use eframe::egui::{include_image, ImageSource};

pub type Img = ImageSource<'static>;

pub fn back() -> Img {
    include_image!("material-symbols/arrow_back_24dp_0000F5_FILL0_wght400_GRAD0_opsz24.svg")
}