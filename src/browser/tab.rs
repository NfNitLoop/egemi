use core::f32;

use eframe::egui::{self, style::ScrollAnimation, text::{CCursor, CCursorRange}, vec2, Align2, Button, Color32, Frame, Image, Key, OpenUrl, ScrollArea, Shadow, Stroke, TextEdit};
use egui_flex::{item, FlexAlign, FlexAlignContent};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use tokio::task::JoinHandle;

use crate::{browser::network::{self, file::{self, FileStatus}, http::HttpLoader, rt, LoadedResource, MultiLoader, SCow}, gemtext::{self, Block}, gemtext_widget::GemtextWidget, svg, widgets::textbox::TextBox};

/// A single tab in the browser.
/// Each tab has its own history and URL.
#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Tab {
    // What the user has currently entered into the location box.
    location: SCow,
    history: Vec<SCow>, // urls

    // TODO: In future I may make this a Box<dyn Widget> so we can swap in other renderers:
    #[serde(skip)]
    document: GemtextWidget,

    #[serde(skip)]
    loading: Option<JoinHandle<network::Result<LoadedResource>>>,

    #[serde(skip)]
    loader: MultiLoader,

    #[serde(skip)]
    shortcuts: Shortcuts,

    #[serde(skip)]
    scroll_to_top: bool,
}

impl Tab {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        self.check_tasks();

        self.location_bar_ui(ui);

        let frame = Frame::new()
            .fill(ui.style().visuals.extreme_bg_color)
            .inner_margin(4.0)
            .outer_margin(0.0)
        ;

        frame.show(ui, |ui| {
            ScrollArea::vertical()
                // .id_salt(&self.location) // No effect?
                .show(ui, |ui| {
                    ui.expand_to_include_rect(ui.available_rect_before_wrap());
                    if self.scroll_to_top {
                        ui.scroll_to_cursor_animation(None, ScrollAnimation::none());
                        self.scroll_to_top = false;
                    }
                    let response = self.document.ui(ui);
                    if let Some(url) = response.link_clicked {
                        self.link_clicked(ui, url);
                    }
                });
        });
    }

    fn location_bar_ui(&mut self, ui: &mut egui::Ui) {
        let frame = Frame::new()
            .fill(Color32::from_rgba_unmultiplied(200, 200, 200, 128))
            .inner_margin(4.0)
            .outer_margin(0.0)
            .stroke(Stroke::new(0.0, Color32::WHITE))
            .shadow(Shadow::default())
        ;

        // Don't pad between location bar & document body:
        let old_spacing = ui.style().spacing.item_spacing.clone();
        ui.style_mut().spacing.item_spacing = vec2(0.0, 0.0);

        frame.show(ui, |ui| {
            let flex = egui_flex::Flex::horizontal()
                .w_full()
                .align_content(FlexAlignContent::Stretch)
                .gap(old_spacing.clone())
            ;
            flex.show(ui, |ui| {
                let back_enabled = self.history.len() > 1;
                let back_svg = Image::new(svg::back())
                    .max_height(18.0)
                    .tint(
                        // Match dark/light(/etc) theme.
                        ui.visuals().text_color()
                    );
                let button = ui.add(item().enabled(back_enabled), Button::new(back_svg));
                if button.clicked() {
                    self.go_back();
                }

                let is_loading = self.is_loading();
                let mut textbox = TextBox::new(self.location.to_mut())
                    .enabled(!is_loading);
                ui.add_widget(item().grow(1.0), &mut textbox);
                if textbox.enter_pressed(ui.ui()) {
                    self.goto_url(self.location.clone());
                } else if textbox.lost_focus() {
                    if let Some(url) = self.history.last().map(Clone::clone) {
                        // !!! I'm surprised I can do this while textbox still has location.to_mut()!?!?
                        self.location = url;
                    }
                } else if self.shortcuts.location_bar(ui.ui()) {
                    textbox.select_all(ui.ui());
                    textbox.request_focus();
                };

                if is_loading {
                    ui.add_ui(item(), |ui| ui.spinner() );
                }              
            });
        });

        ui.style_mut().spacing.item_spacing = old_spacing;    

    }

    // Full URL entered in location bar, or set by app. 
    pub fn goto_url(&mut self, url: SCow) {
        if let Some(loading) = self.loading.take() {
            loading.abort();
            // (drop)
        }

        let url: SCow = url.into();

        self.history.push(url.clone());
        self.location = url.clone();

        if url == BuiltinUrl::ABOUT {
            self.set_gemtext(ABOUT_EGEMI.trim_start());
            return
        }
        
        let handle = self.loader.fetch(url);
        self.loading = Some(handle);        
    }

    pub fn link_clicked(&mut self, ui: &egui::Ui, url: String) {
        // Handle browser+ links:
        if let Some(url) = url.strip_prefix("browser+") {
            ui.ctx().open_url(OpenUrl{
                url: url.into(),
                new_tab: false
            });
            return;
        }

        if let Ok(joined) = url_join(&self.location, &url) {
            self.goto_url(joined.to_string().into());
            return;
        }
                
        // TODO: Relative resolution.
        self.goto_url(url.into());
    }

    pub fn go_back(&mut self) {
        if self.history.len() <= 1 {
            eprintln!("Warning: Tried to go back with no history. (Button should be disabled.)");
            return;
        }

        // TODO: Forward button support.
        // The top of history is the current URL:
        self.history.pop().expect("drop current URL");

        // Easier to just pop the old URL and nagivate to it again like it's the first time:
        let url = self.history.pop().expect("previous url");
        self.goto_url(url);
    }

    fn set_gemtext(&mut self, text: &str) {
        let parser = gemtext::Options::default();
        let blocks = match parser.parse(text) {
            Ok(blocks) => blocks,
            Err(err) => {
                let text = format!("{err:#?}");
                vec![
                    Block::Heading { level: 1, text: "Gemtext Parse Error".into() },
                    Block::Text(String::new()),
                    Block::Text(text),
                ]
            },
        };
        self.document.set_blocks(blocks);
        self.scroll_to_top = true;
    }

    fn set_plaintext(&mut self, text: &str) {
        let blocks: Vec<Block> = text.lines().map(|line| Block::Text(line.into())).collect();
        self.document.set_blocks(blocks);
        self.scroll_to_top = true;
    }
    
    /// Check if any async tasks completed. Right now, this is just whether a page loaded.
    fn check_tasks(&mut self) {
        let Some(loading) = &self.loading else {
            return;
        };
        if !loading.is_finished() {
            return;
        }
        let Some(loading) = self.loading.take() else {
            return; // Wha? We know it should be some!
        };
        let fut = async {
            loading.await
        };
        
        // We expect this not to block (long) because the task is finished already:
        let result = rt().block_on(fut);

        let result = match result {
            Ok(ok) => ok,
            Err(err) => {
                let msg = format!("{err:#?}");
                self.set_gemtext(&msg);
                return;
            }
        };

        let loaded = match result {
            Ok(ok) => ok,
            Err(err) => {
                self.render_err(err);
                return;
            },
        };

        if !loaded.status.ok() {
            use network::Status::*;
            match loaded.status {
                HttpStatus { code } => {
                    let text = format!("## HTTP {code}") 
                        + "\n"
                        + "\nSee:"
                        + "\n=> https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Status";
                    self.set_gemtext(&text);
                    return;
                },
                FileStatus(file::FileStatus::DirNeedsSlash) => {
                    // continue to output.
                },
                FileStatus(status) => {
                    let text = format!("## {status:?}");
                    self.set_gemtext(&text);
                    return;
                },
            }            
        }

        let is_text = match &loaded.content_type {
            None => {
                // Not actually sure, but show text anyway?
                true
            },
            Some(content) => {
                content.type_().as_str() == "text"
            }
        };

        if !is_text {
            let content = loaded.content_type
                .map(|it| format!("{it}"))
                .unwrap_or_else(|| format!("<unknown>"));
            let msg = format!("## Unsupported Content-Type\n\n")
                + &format!("Content-Type: {content}\n")
                + "is not yet supported.\n\n"
                + "=> browser+" + &self.encoded_location() + " Open in browser?"
            ;

            self.set_gemtext(&msg);
            return;
        }

        let body = match loaded.body {
            network::Body::Bytes(_cow) => "binary data".into(),
            network::Body::Text(cow) => cow,
        };

        let is_gemtext = loaded.content_type.map(|it| it.essence_str() == "text/gemini").unwrap_or(false);
        if is_gemtext {
            self.set_gemtext(&body);
        } else {
            self.set_plaintext(&body);
        }
    }
    
    fn is_loading(&self) -> bool {
        let Some(loading) = &self.loading else {
            return false;
        };
        !loading.is_finished()
    }
    
    fn render_err(&mut self, err: network::Error){
        use network::Error::*;
        match err {
            MissingContentType 
            | MimeParseError(_) 
            | UnsupportedUrlScheme(_)
            | InvalidUrl(_)
            | IoError(_)
            | UnsupportedContentType(_)
            | Unknown(_) => {
                // Just show default error.
            },
            UnrequestedContentType(mime) => {
                let text = format!("## Unrequested Content-Type\n\n```\nContent-Type: {mime}\n```\n")
                + "=> browser+" + &self.encoded_location() + " Open in web browser";
                self.set_gemtext(&text);
                return;
            },
        };
        
        let msg = format!("{err:#?}");
        self.set_gemtext(&msg);
        return;
    }

    fn encoded_location(&self) -> String {
        // TODO: Proper URLencode. Avoid if unnecessary.
        self.location.replace(" ", "%20")
    }

}

fn url_join(location: &str, url: &str) -> Result<Url, ()> {
    let base = Url::parse(location).map_err(|_| ())?;
    let joined = base.join(url).map_err(|_| ())?;
    Ok(joined)
}

struct BuiltinUrl;
impl BuiltinUrl {
    const ABOUT: &str = "about:egemi";
}


const ABOUT_EGEMI: &str = include_str!("../../welcome.gmi");


/// A place to check whether keyboard shortcuts were pressed.
/// May be configurable in the future.
#[derive(Default, Debug)]
struct Shortcuts;

impl Shortcuts {
    fn location_bar(&self, ui: &egui::Ui) -> bool {
        ui.input(|i| {
            i.key_pressed(Key::L)
            && (i.modifiers.ctrl || i.modifiers.command)
        })
    }
}