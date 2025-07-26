pub mod fonts;
mod network;
mod tab;

use eframe::{egui::{self, global_theme_preference_buttons, CentralPanel, Color32, FontData, FontFamily, Frame, MenuBar, TopBottomPanel}, epaint::text::{FontInsert, InsertFontFamily}, App, NativeOptions};
use serde::{Deserialize, Serialize};

use crate::{browser::{fonts::load_fonts, tab::Tab}, gemtext_widget::{self, GemtextWidget}, DynResult};

pub fn main(url: String) -> eframe::Result {
    let opts = NativeOptions {
        persist_window: true,
        ..Default::default()
    };

    eframe::run_native(
        "egemi",
        opts,
        Box::new(move |c| {
            let mut app = Browser::new(c);
            app.goto_url(url);
            let app = Box::new(app);
            Ok(app)
        }),
    )
}

/// The main browser window.
/// May eventually support multiple tabs. For now, there's only ever one, which fills the whole screen.
#[derive(Serialize, Deserialize, Debug)]
struct Browser {
    tab: Tab
}

impl Browser {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        load_fonts(cc);

        // TODO: Better themes:
        gemtext_widget::Style::config(&cc.egui_ctx);

        Self { 
            tab: Tab::default()
        }
    }
    
    fn goto_url(&mut self, url: String) {
        self.tab.goto_url(url.into());
    }

    fn menu_bar(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("About").clicked() {
                    self.goto_url("about:egemi".into());
                }
                global_theme_preference_buttons(ui);
                if ui.button("Quit").clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            })
        });
    }
}



impl App for Browser {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        TopBottomPanel::top("top panel").show(ctx, |ui| self.menu_bar(ctx, ui));

        let frame = Frame::new()
            .outer_margin(0.0)
            .inner_margin(0.0)
        ;

        CentralPanel::default()
            .frame(frame)
            .show(ctx, |ui| {
                self.tab.ui(ui);
            });
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }
}