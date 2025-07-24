//! A hacky little interactive gemtext editor.
//! Mostly used to debug gemtext parsing/rendering.

use eframe::{egui::{self, Context, ScrollArea, TextEdit, TextStyle}, Frame, NativeOptions};

use crate::{gemtext::{self, Block}, gemtext_widget::{self, GemtextWidget}};

pub fn main() -> eframe::Result {
    let opts = NativeOptions {

        ..Default::default()
    };

    eframe::run_native(
        "egemi",
        opts,
        Box::new(|c| {
            let app = App::new(c);
            let app = Box::new(app);
            Ok(app)
        }),
    )
}

struct App {
    text: String,
    gemtext: GemtextWidget,
}

impl eframe::App for App {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        self.menu(ctx);
        egui::CentralPanel::default().show(ctx, |ui| self.body(ui));
    }
}


impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        gemtext_widget::Style::config(&cc.egui_ctx);
        Self {
            text: String::from("Edit me!"),
            gemtext: GemtextWidget::default(),
        }
    }

    fn menu(&mut self, ctx: &Context) {
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("egemi", |ui| {
                    egui::widgets::global_theme_preference_buttons(ui);
                    let clicker = ui.button("TODO: Something here");
                    if clicker.clicked() {
                        println!("Clicked");
                    }
                });
                egui::warn_if_debug_build(ui);
            });
        });
    }
    
    fn body(&mut self, ui: &mut egui::Ui) {
        ui.columns(2, |ui| {
            self.left_pane_ui(&mut ui[0]);
            self.right_pane_ui(&mut ui[1]);
        });

    }
    
    fn left_pane_ui(&mut self, ui: &mut egui::Ui) {
        ScrollArea::vertical().id_salt("left").show(ui, |ui| {
            let edit = TextEdit::multiline(&mut self.text).font(TextStyle::Monospace);
            let response = ui.add_sized(ui.available_size(), edit);
            if response.changed() {
                self.rerender();
            }
        });
    }

    fn right_pane_ui(&mut self, ui: &mut egui::Ui) {
        // Render gemtext:
        ScrollArea::vertical().id_salt("right").show(ui, |ui| {
            self.gemtext.ui(ui);
        });

    }

    fn rerender(&mut self) {
        let result = gemtext::Options::default().parse(&self.text);
        if let Ok(blocks) = result {
            self.gemtext.set_blocks(blocks);
        } else {
            self.gemtext.set_blocks(vec![
                Block::Text(format!("Error parsing"))
            ]);
        }
    }
}

