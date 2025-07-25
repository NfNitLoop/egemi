use core::f32;
use std::sync::Arc;

use eframe::egui::{self, vec2, Button, Color32, Frame, Key, OpenUrl, ScrollArea, Shadow, Stroke, TextEdit};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use tokio::task::JoinHandle;

use crate::{browser::network::{self, http::HttpLoader, LoadedResource, MultiLoader, SCow}, gemtext::{self, Block}, gemtext_widget::GemtextWidget};

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
    loader: MultiLoader
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
            ui.expand_to_include_rect(ui.available_rect_before_wrap());
            ScrollArea::vertical()
                // .id_salt(&self.location) // No effect?
                .show(ui, |ui| {
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

        // Don't pad before next item:
        let old_padding = ui.style().spacing.item_spacing.clone();     
        ui.style_mut().spacing.item_spacing = vec2(0.0, 0.0);


        frame.show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.style_mut().spacing.item_spacing = old_padding;    
                let back_enabled = self.history.len() > 1;
                if ui.add_enabled(back_enabled, Button::new("⬅")).clicked() {
                    self.go_back();
                }
                let is_loading = self.is_loading();
                if is_loading {
                    ui.spinner();
                }
                let textbox = TextEdit::singleline(&mut self.location)
                    .desired_width(f32::INFINITY);

                let loc = ui.add_enabled(!is_loading, textbox);
                if loc.lost_focus() { // user pressed enter OR tabbed away.
                    if ui.input(|i| i.key_pressed(Key::Enter)) {
                        self.goto_url(self.location.clone());
                    } else {
                        if let Some(url) = self.history.last().map(Clone::clone) {
                            self.location = url;
                        }
                    }
                }
            });
        });    

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
        
        let rt = tokio::runtime::Builder::new_current_thread().build().expect("current-thread");
        let result = rt.block_on(fut);

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
            let status = &loaded.status;
            let text = format!("## {status}") 
                + "\n"
                + "\nSee:"
                + "\n=> https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Status";
            self.set_gemtext(&text);
            return;
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

        self.set_gemtext(&body);
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


const ABOUT_EGEMI: &str = r#"
# eGemi

An egui browser for Gemini Text ("Gemtext") files.

eGemi's main differentiator from other Gemini browsers is that it can also read gemtext from HTTP!

This is the result of my thinking from:
=> https://www.nfnitloop.com/blog/2025/06/project-gemini/ Thoughts on Project Gemini

## Features

* Can read gemtext over HTTP(S) as well as Gemini protocol.
* Sends HTTP Accept headers requesting text/gemtext, text/markdown, and text/plain.

## Intentionally Missing Features

While using HTTP, we can maintain some of the benefits of using Gemini Protocol by just not implementing the parts of HTTP that are abused:

* HTTP Cookies (No tracking)
* JavaScript (No popups)
* Loading secondary resources on a page, like CSS, Images, fonts, etc. (No tracking pixels, or cross-site cookies.)
* Automatic redirects (click tracking)

## Possible Future Features

* Read a local bookmarks.gmi or home.gmi for quick access to common sites.
* Multiple browser tabs.
* Render a safe subset of Markdown. (Currently gets parsed/rendered as Gemtext. YMMV.)
* Options for Gemtext rendering. (ex: show more/less native syntax.) 
* Nicer and/or customizable themes. Maybe per-domain. Right now, we've got egui's built-in light/dark themes.
* Caching (likely only in-memory.) 
* Rendering of simple HTML. (think: "Reader" mode from iOS Safari.)
* RSS support.

### See:
=> https://raw.githubusercontent.com/NfNitLoop/gemi/refs/heads/main/README.md Gemi
=> browser+https://github.com/nfnitloop/egemi @nfnitloop/egemi on GitHub
=> https://nfnitloop.com
=> gemini://geminiprotocol.net/
"#;
