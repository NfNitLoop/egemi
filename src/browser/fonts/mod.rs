use eframe::{egui::{self, FontData, FontDefinitions, FontFamily}, epaint::text::{FontInsert, InsertFontFamily}};

pub fn load_fonts(cc: &eframe::CreationContext) {
    cc.egui_ctx.set_fonts(FontDefinitions::empty());
    noto_sans(cc);
    noto_sans_mono(cc);
    noto_emoji(cc);

    add_prop(cc, "NotoSansJP", include_bytes!("NotoSansJP-VariableFont_wght.ttf"));
    add_prop(cc, "NotoSansKR", include_bytes!("NotoSansKR-VariableFont_wght.ttf"));
    add_prop(cc, "NotoSansSC", include_bytes!("NotoSansSC-VariableFont_wght.ttf"));
    add_prop(cc, "NotoSansTC", include_bytes!("NotoSansTC-VariableFont_wght.ttf"));
}

fn add_prop(cc: &eframe::CreationContext, name: &str, bytes: &'static [u8]){
    cc.egui_ctx.add_font(FontInsert{
        name: name.into(),
        data: FontData {
            font: bytes.into(),
            index: 0,
            tweak: Default::default(),
        },
        families: vec![
            InsertFontFamily{
                family: FontFamily::Proportional,
                priority: egui::epaint::text::FontPriority::Lowest,
            },
            InsertFontFamily{
                family: FontFamily::Monospace,
                priority: egui::epaint::text::FontPriority::Lowest,
            },
        ]
    })
}


fn noto_sans(cc: &eframe::CreationContext) {
    let bytes = include_bytes!("NotoSans-Variable.ttf");
    let name = "Noto Sans";
    cc.egui_ctx.add_font(FontInsert{
        name: name.into(),
        data: FontData {
            font: bytes.into(),
            index: 0,
            tweak: Default::default(),
        },
        families: vec![
            InsertFontFamily{
                family: FontFamily::Proportional,
                priority: egui::epaint::text::FontPriority::Highest,
            },
        ]
    })
}

fn noto_sans_mono(cc: &eframe::CreationContext) {
    let bytes = include_bytes!("NotoSansMono-Variable.ttf");
    let name = "Noto Sans Mono";
    cc.egui_ctx.add_font(FontInsert{
        name: name.into(),
        data: FontData {
            font: bytes.into(),
            index: 0,
            tweak: Default::default(),
        },
        families: vec![
            InsertFontFamily{
                family: FontFamily::Monospace,
                priority: egui::epaint::text::FontPriority::Highest,
            },
        ]
    })
}

// Sadly, egui doesn't support color fonts yet:
fn noto_emoji(cc: &eframe::CreationContext) {
    let bytes = include_bytes!("noto-emoji/NotoEmoji-Variable.ttf");
    let name = "NotoEmoji";
    cc.egui_ctx.add_font(FontInsert{
        name: name.into(),
        data: FontData {
            font: bytes.into(),
            index: 0,
            tweak: Default::default(),
        },
        families: vec![
            InsertFontFamily{
                family: FontFamily::Proportional,
                priority: egui::epaint::text::FontPriority::Lowest,
            },
            InsertFontFamily{
                // I don't think this is technically monospace, but it's better than no emoji?
                family: FontFamily::Monospace,
                priority: egui::epaint::text::FontPriority::Lowest,
            },
        ]
    })
}