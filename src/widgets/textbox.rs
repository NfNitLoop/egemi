
use eframe::egui::{self, text::{CCursor, CCursorRange}, text_edit::TextEditOutput, Key, TextEdit as TE, Ui, Widget};

/// Implements select_all on a TextEdit::singleline().
/// So that you don't have to grab TextEditOutput yourself.
/// Which is a PITA because Widget::ui() doesn't return it.
/// And that's what egui_flex wants.
pub struct TextBox<'a> {
    value: &'a mut String,
    last_out: Option<TextEditOutput>,
    enabled: bool,
}


impl <'a> TextBox<'a> {
    pub fn new(buffer: &'a mut String) -> Self {
        Self {
            value: buffer,
            last_out: None,
            enabled: true,
        }
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn select_all(&self, ui: &egui::Ui) {
        // This feels like such a hack!

        let Some(output) = &self.last_out else {
            // Have to render before we can select_all.
            return;
        };

        let range = 0..self.value.len();
        let mut state = output.state.clone();
        state.cursor.set_char_range(Some(CCursorRange {
            // Note! "primary" is where selection "ended" and will be where the cursor appears.
            secondary: CCursor { index: range.start, prefer_next_row: true },
            primary: CCursor { index: range.end, prefer_next_row: true },
            h_pos: None
        }));
        state.store(ui.ctx(), output.response.id);
    }
    
    pub fn lost_focus(&self) -> bool {
        let Some(out) = &self.last_out else { return false; };
        out.response.lost_focus()
    }
    
    /// Note egui causes lost_focus when enter is pressed, so make sure to check this
    /// condition before lost_focus().
    pub(crate) fn enter_pressed(&self, ui: &egui::Ui) -> bool {
        if !self.lost_focus() { return false }

        ui.input(|i| {
            i.key_pressed(Key::Enter)
        })
    }
    
    pub(crate) fn request_focus(&self) {
        let Some(out) = &self.last_out else { return };
        out.response.request_focus();
    }
}


impl <'a> Widget for &mut TextBox<'a> {
    /// Required to be a Widget. But doesn't return TextOut.
    /// So we save it for later use.
    fn ui(self, ui: &mut Ui) -> egui::Response {
        let response = ui.add_enabled_ui(self.enabled, |ui| {
            let out = TE::singleline(self.value).show(ui);
            let response = out.response.clone();
            self.last_out = Some(out);
            response
        });

        response.inner
    }
}