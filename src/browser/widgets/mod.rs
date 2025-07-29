pub mod markdown;

use std::fmt::Debug;

use eframe::egui::Ui;


/// Returned by a document renderer
pub struct DocumentResponse {
    pub link_clicked: Option<String>
}

/// Responsible for rendering a document within a tab.
pub trait DocWidget: Debug {
    fn ui(&mut self, ui: &mut Ui) -> DocumentResponse;

    // TODO: update theme.
}

// TODO: Necessary?
// impl <'a, T> DocWidget for &'a mut Box<T> where &'a mut T: DocWidget {
//     fn ui(self, ui: &mut Ui) -> DocumentResponse {
//         self.as_mut().ui(ui)
//     }
// }