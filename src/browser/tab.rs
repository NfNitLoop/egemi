use core::f32;

use eframe::egui::{self, vec2, Button, Color32, Frame, Key, OpenUrl, ScrollArea, Shadow, Stroke, TextEdit};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use tokio::task::JoinHandle;

use crate::{browser::network::{self, HttpLoader, LoadedResource, SCow}, gemtext::{self, Block}, gemtext_widget::GemtextWidget};

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
        
        let loader = HttpLoader::default();
        let handle = loader.fetch(&url);
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
                let msg = format!("{err:#?}");
                self.set_gemtext(&msg);
                return;
            },
        };



        let is_gemtext = match &loaded.content_type {
            None => {
                // Not actually sure, but show text anyway?
                true
            },
            Some(content) => {
                content.starts_with("text/gemini")
            }
        };

        if !is_gemtext {
            let content = loaded.content_type.unwrap_or_else(|| "<unknown>".to_string().into());
            let url = String::from("browser+") + &self.location.replace(" ", "%20"); // TODO: proper url encode.
            let msg = format!("Content-Type: {content}\n")
                + "is not yet supported.\n"
                + &format!("=> {url} Open in browser?")
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


const ABOUT_EGEMI: &str = "
# eGemi

An egui browser for Gemini texts via http(s):// and gemini://

### See:
=> https://nfnitloop.com
=> https://geminiprotocol.net

### See Also (TODO)
=> browser+https://github.com/nfnitloop/egemi TODO: @nfnitloop/egemi on GitHub

";
