pub mod fonts;
mod network;
mod tab;

use std::path::PathBuf;

use eframe::{egui::{self, global_theme_preference_buttons, gui_zoom::zoom_menu_buttons, CentralPanel, Frame, MenuBar, TopBottomPanel, ViewportBuilder}, App, NativeOptions};
use egui_extras::install_image_loaders;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{browser::{fonts::load_fonts, tab::Tab}, gemtext_widget::{self}};

pub fn main(url: String) -> eframe::Result {
    let opts = NativeOptions {
        persist_window: true,
        ..Default::default()
    };
    let url = try_file_url(url);

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

fn try_file_url(url: String) -> String {
    if Url::parse(&url).is_ok() { 
        return url;
    }
    let Ok(path) = PathBuf::from(&url).canonicalize() else {
        return url;
    };

    let new_url = if path.is_dir() { 
        Url::from_directory_path(path)
    } else {
        Url::from_file_path(path)
    };
    let Ok(new_url) = new_url else {
        return url;
    };
    return new_url.to_string();

}

/// The main browser window.
/// May eventually support multiple tabs. For now, there's only ever one, which fills the whole screen.
#[derive(Serialize, Deserialize, Debug)]
struct Browser {
    tab: Tab,

    // Allows us to toggle menu on/off
    show_menu: bool,
}

impl Browser {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        install_image_loaders(&cc.egui_ctx);
        load_fonts(cc);

        // TODO: Better themes:
        gemtext_widget::Style::config(&cc.egui_ctx);

        Self { 
            tab: Tab::default(),
            // On first load:
            show_menu: false,
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
            });

            // Not really meant to be rendered in a menu. (Closes w/ each click)
            // ui.menu_button("Settings", |ui| {
            //     ctx.settings_ui(ui);
            // });

            ui.menu_button("Zoom", |ui| {
                zoom_menu_buttons(ui);
            });
        });
    }
}



impl App for Browser {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        TopBottomPanel::top("top panel")
            .show_animated(ctx, self.show_menu, |ui| {
                self.menu_bar(ctx, ui)
            });

        let frame = Frame::new()
            .outer_margin(0.0)
            .inner_margin(0.0)
        ;

        CentralPanel::default()
            .frame(frame)
            .show(ctx, |ui| {
                let tab = self.tab.show(ui);
                if tab.toggle_menu {
                    self.show_menu = !self.show_menu;
                }

            });
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }
}