use eframe::egui::{self, include_image, Button, Image, ImageSource, Ui, Widget};


pub fn back() -> SvgButton { SvgButton{ img: include_image!("material-symbols/arrow_back.svg") } }
pub fn forward() -> SvgButton { SvgButton{ img: include_image!("material-symbols/arrow_forward.svg") } }
pub fn menu() -> SvgButton { SvgButton{ img: include_image!("material-symbols/menu.svg") } }
pub fn reload() -> SvgButton { SvgButton { img: include_image!("material-symbols/refresh.svg") } }

pub struct SvgButton {
    img: Img,
}

pub type Img = ImageSource<'static>;

impl Widget for SvgButton {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        let img = Image::new(self.img)
            .max_height(18.0)
            .tint(
                // Match dark/light(/etc) theme.
                ui.visuals().text_color(),
            );
        ui.add(Button::new(img))
    }
}
