use eframe::{
    egui::{self, Context, ScrollArea, TextEdit}, Frame, NativeOptions
};

mod gemtext;

fn main() -> eframe::Result {
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
    rendered: String,
}

impl eframe::App for App {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        self.menu(ctx);
        egui::CentralPanel::default().show(ctx, |ui| self.body(ui));
    }
}


impl App {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            text: String::from("Edit me!"),
            rendered: String::new(),
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
            let edit = TextEdit::multiline(&mut self.text);
            let response = ui.add_sized(ui.available_size(), edit);
            if response.changed() {
                self.rerender();
            }
        });
    }

    fn right_pane_ui(&mut self, ui: &mut egui::Ui) {
        ScrollArea::vertical().id_salt("right").show(ui, |ui| {
            let mut readonly = self.rendered.as_str();
            let edit = TextEdit::multiline(&mut readonly);
            let response = ui.add_sized(ui.available_size(), edit);
        });
    }

    fn rerender(&mut self) {
        let result = gemtext::Options::default().parse(&self.text);
        self.rendered = format!("{result:#?}")
    }



}

