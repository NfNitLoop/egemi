use eframe::{egui::{self, vec2, Color32, FontId, Frame, Link, RichText, Sense, TextStyle, Ui, UiBuilder, Vec2}, epaint::MarginF32};

use crate::gemtext::Block;

#[derive(Default, Debug)]
pub struct GemtextWidget {
    blocks: Vec<Block>,

    // Should hard-code to false until this bug is fixed:
    // https://github.com/emilk/egui/issues/1272
    justify: bool,

    link_clicked: Option<String>, // "url", but may not parse as such.
}

impl GemtextWidget {
    pub fn ui(&mut self, ui: &mut Ui) -> Response {
        // Assuming we're in a top-down layout, because that's all that really makes sense:
        let mut layout = *ui.layout();
        layout.cross_justify = self.justify;

        ui.with_layout(layout, |ui| {
            // It turns out, the text renderer puts plenty of space.
            // But leaving spacing around every line, especially blank lines, made for a very whitespace-heavy feel.
            ui.spacing_mut().item_spacing = Vec2::ZERO;

            self.render(ui)
        });

        Response {
            link_clicked: self.link_clicked.take(),
        }
    }

    fn render(&mut self, ui: &mut Ui) {
        let mut line_num: u32 = 0;
        for block in &self.blocks {
            line_num += 1;
            match block {
                Block::Heading { level, text } => {
                    let is_title = line_num == 1 && *level == 1;
                    let style = if is_title { Style::title() } else { Style::heading(*level) };
                    let rt = RichText::new(text).text_style(style).strong();
                    if is_title {
                        ui.vertical_centered(|ui| {
                            ui.label(rt);
                        });
                    } else {
                        ui.label(rt);
                    }
                },
                Block::Text(text) => {
                    ui.label(text);
                },
                Block::ListItem { text } => {
                    ui.horizontal_top(|ui| {
                        ui.label(" • ");
                        ui.vertical(|ui| {
                            ui.label(text);
                        })
                    });
                },
                Block::BlockQuote { lines } => {
                    block_quote(ui, lines);
                },
                Block::CodeFence { meta: _, lines } => {
                    for line in lines {
                        // ui.monospace(line);
                        let rt = RichText::new(line).text_style(Style::mono());
                        ui.label(rt);
                    }
                },
                Block::Link { url, text } => {
                    let visible = if text.is_empty() { url } else { text };
                    let link = Link::new(visible);
                    let response = ui.add(link);
                    if response.clicked() {
                        self.link_clicked = Some(url.clone());
                    }
                    response.on_hover_ui(|ui| {
                        ui.monospace(url);
                    });
                },
            }
        }
    }

    pub fn set_blocks(&mut self, blocks: Vec<Block>) {
        self.blocks = blocks;
    }
}

/// Returned by [`GemtextWidget::ui`] so you can access events.
pub struct Response {
    pub link_clicked: Option<String>
}


fn block_quote(ui: &mut Ui, lines: &Vec<Block>) {
    let builder = UiBuilder::new();
    let row_height = ui.text_style_height(&TextStyle::Body);
    let left_margin = MarginF32{ left: row_height / 2.0, ..Default::default() };
    let response = ui.scope_builder(builder, |ui| {
        let frame = Frame::new()
            .outer_margin(left_margin);
        frame.show(ui, |ui| {
            for line in lines {
                if let Block::Text(line) = line {
                    ui.label(line);
                }
            }
        });

    });
    let rect = response.response.rect;
    ui.painter().line_segment(
        [rect.left_top(), rect.left_bottom()],
        (1.0, ui.visuals().weak_text_color()),
    );
}


pub struct Style;

impl Style {
    // Custom named styles. w/ a util to config them.
    pub fn heading(level: u8) -> TextStyle {
        if level <= 1 {
            Self::h1()
        } else if level == 2 {
            Self::h2()
        } else {
            Self::h3()
        }
    }

    pub fn h1() -> TextStyle { Self::named("H1") }
    pub fn h2() -> TextStyle { Self::named("H2") }
    pub fn h3() -> TextStyle { Self::named("H3") }
    pub fn mono() -> TextStyle { Self::named("gemtext-mono") }

    /// The first H1 in a Gemtext is the page Title:
    pub fn title() -> TextStyle { Self::named("Title") }

    fn named(name: &str) -> TextStyle { TextStyle::Name(name.into()) }

    pub fn config(ctx: &egui::Context) {
        use egui::FontFamily::{Proportional, Monospace};
        let body_size = ctx.style().text_styles.get(&TextStyle::Body).expect("TextStyle::Body should always be present").size;
        ctx.all_styles_mut(|style| {
            style.text_styles.entry(Self::title()).or_insert(FontId::new(body_size * 2.0, Proportional));
            style.text_styles.entry(Self::h1()).or_insert(FontId::new(body_size * 2.0, Proportional));
            style.text_styles.entry(Self::h2()).or_insert(FontId::new(body_size * 1.5, Proportional));
            style.text_styles.entry(Self::h3()).or_insert(FontId::new(body_size * 1.2, Proportional));            
            style.text_styles.entry(Self::mono()).or_insert(FontId::new(body_size * 0.8, Monospace));            
        });
    }
}