
use eframe::egui::{self, style::ScrollAnimation, vec2, Button, Color32, Frame, Image, Key, Modifiers, OpenUrl, ScrollArea, Shadow, Stroke, Ui, Vec2};
use egui_flex::{item, FlexAlignContent};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use tokio::task::JoinHandle;

use crate::{browser::{network::{self, file::{self}, rt, LoadedResource, MultiLoader, SCow}, widgets::{markdown, DocWidget}}, gemtext::{self, Block}, gemtext_widget::GemtextWidget, svg::{self, menu}, widgets::textbox::TextBox};

/// A single tab in the browser.
/// Each tab has its own history and URL.
#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Tab {
    // What the user has currently entered into the location box.
    location: SCow,

    // urls:
    history: Vec<SCow>, 
    forward_history: Vec<SCow>,

    #[serde(skip)]
    document: Option<Box<dyn DocWidget>>,

    #[serde(skip)]
    loading: Option<JoinHandle<network::Result<LoadedResource>>>,

    #[serde(skip)]
    loader: MultiLoader,

    #[serde(skip)]
    shortcuts: Shortcuts,

    #[serde(skip)]
    scroll_to_top: bool,

    #[serde(skip)]
    toggle_menu: bool,
}

impl Tab {
    pub fn show(&mut self, ui: &mut egui::Ui) -> TabResponse {
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
                    let Some(document) = self.document.as_mut()  else {
                        return;
                    };
                    let doc_ref = document.as_mut();
                    let response = doc_ref.ui(ui);
                    if let Some(url) = response.link_clicked {
                        self.link_clicked(ui, url);
                    }
                });
        });

        TabResponse {
            toggle_menu: { let tm = self.toggle_menu; self.toggle_menu = false; tm },
        }
    }

    fn location_bar_ui(&mut self, ui: &mut egui::Ui) {
        let frame_pad = 4.0;
        let frame = Frame::new()
            .fill(Color32::from_rgba_unmultiplied(200, 200, 200, 128))
            .inner_margin(frame_pad)
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
                .gap(Vec2::splat(frame_pad))
            ;
            flex.show(ui, |ui| {
                let is_loading = self.is_loading();

                let back_enabled = self.history.len() > 1;
                let back = ui.add_widget(item().enabled(back_enabled), svg::back());
                if back.inner.clicked() {
                    self.go_back();
                }

                let fw_enabled = !self.forward_history.is_empty();
                let fw = ui.add_widget(item().enabled(fw_enabled), svg::forward());
                if fw.inner.clicked() {
                    self.go_forward();
                }

                let reload = ui.add_widget(item().enabled(!is_loading), svg::reload());
                if reload.inner.clicked() || self.shortcuts.reload(ui.ui()) {
                    self.reload();
                }

                let mut textbox = TextBox::new(self.location.to_mut())
                    .enabled(!is_loading);
                ui.add_widget(item().grow(1.0).shrink(), &mut textbox);
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

                let toggle_menu = ui.add_widget(item(), menu());
                if toggle_menu.inner.clicked() {
                    self.toggle_menu = true;
                }
            });
        });

        ui.style_mut().spacing.item_spacing = old_spacing;    
    }

    // Full URL entered in location bar, or set by app.
    pub fn goto_url(&mut self, url: SCow) {
        let fw_history_matches = self.forward_history.last().map(|it| it == &url).unwrap_or(false);
        if fw_history_matches {
            self.forward_history.pop();
        } else {
            self.forward_history.clear();
        }

        self.load_url(url);
    }

    /// Like goto_url(), but does NOT clear the forward_history.
    /// You should prefer goto_url() for most cases.
    fn load_url(&mut self, url: SCow) {
        if let Some(loading) = self.loading.take() {
            loading.abort();
            // (drop)
        }

        let url: SCow = url.into();

        self.history.push(url.clone());
        self.location = url.clone();

        // TODO: Move the builtin loading to its own network/ loader module.
        for builtin in BuiltinUrl::ALL {
            if builtin.url == url.as_ref() {
                self.set_gemtext(builtin.text);
                return;
            }
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

        // The top of history is the current URL:
        let current_url = self.history.pop().expect("drop current URL");
        self.forward_history.push(current_url);

        // Easier to just pop the old URL and nagivate to it again like it's the first time:
        let url = self.history.pop().expect("previous url");
        self.load_url(url);
    }

    pub fn go_forward(&mut self) {
        let Some(next_url) = self.forward_history.pop() else {
            eprintln!("Warning: Clicked forward button when no fw history available.");
            return;
        };

        self.load_url(next_url);
    }

    pub fn reload(&mut self) {
        // Right now there's no caching, so just 'goto' this URL again.
        // When there's caching, we'll need to clear/invalidate cache first. Or fetch & replace.
        if let Some(url) = self.history.pop() {
            self.goto_url(url);
        }
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
        let mut new_doc = GemtextWidget::default();
        new_doc.set_blocks(blocks);
        self.document = Some(Box::new(new_doc));
        self.scroll_to_top = true;
    }

    fn set_plaintext(&mut self, text: &str) {
        let blocks: Vec<Block> = text.lines().map(|line| Block::Text(line.into())).collect();
        let mut new_doc = GemtextWidget::default();
        new_doc.set_blocks(blocks);
        self.document = Some(Box::new(new_doc));        self.scroll_to_top = true;
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

        let essence = loaded.content_type.as_ref().map(|it| it.essence_str());
        if let Some("text/gemini") = essence {
            self.set_gemtext(&body);
        } else if let Some("text/html") = essence {
            self.render_html(body);
        } else if let Some("text/markdown") = essence {
            self.render_markdown(body)
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
            e @ ResponseTooBig{..} => {
                let text = format!("## Response too big\n\n{e:?}");
                self.set_gemtext(&text);
                return;
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
    
    fn render_html(&mut self, body: SCow) {
        let new_doc = markdown::MarkdownWidget::for_html(&body);
        self.document = Some(Box::new(new_doc));
    }

    fn render_markdown(&mut self, body: SCow) {
        let new_doc = markdown::MarkdownWidget::for_md(&body);
        self.document = Some(Box::new(new_doc));
    }
}


pub struct TabResponse {
    pub toggle_menu: bool
}

fn url_join(location: &str, url: &str) -> Result<Url, ()> {
    let base = Url::parse(location).map_err(|_| ())?;
    let joined = base.join(url).map_err(|_| ())?;
    Ok(joined)
}

struct BuiltinUrl {
    url: &'static str,
    text: &'static str,
}
impl BuiltinUrl {
    const ABOUT: Self = Self {
        url: "about:egemi",
        text: include_str!("../../welcome.gmi")
    };
    const CHANGELOG: Self = Self {
        url: "about:changelog",
        text: include_str!("../../changelog.gmi")
    };

    const ALL: &'static [BuiltinUrl] = &[
        Self::ABOUT,
        Self::CHANGELOG,
    ];
}



/// A place to check whether keyboard shortcuts were pressed.
/// May be configurable in the future.
#[derive(Default, Debug)]
struct Shortcuts;

impl Shortcuts {
    fn location_bar(&self, ui: &egui::Ui) -> bool {
        ui.input_mut(|i| {
            i.consume_key(Modifiers::COMMAND, Key::L)
        })
    }

    fn reload(&self, ui: &Ui) -> bool {
        ui.input_mut(|i| {
            i.consume_key(Modifiers::COMMAND, Key::R)
        })
    }
}