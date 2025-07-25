use eframe::{egui::{self, vec2, Align, FontId, Frame, Layout, Link, RichText, TextStyle, Ui, UiBuilder}, epaint::MarginF32};

use crate::gemtext::Block;

#[derive(Default, Debug)]
pub struct GemtextWidget {
    blocks: Vec<Block>,

    // Should hard-code to false until this bug is fixed:
    // https://github.com/emilk/egui/issues/1272
    justify: bool,


    keep_prefix_link: bool,

    link_clicked: Option<String>, // "url", but may not parse as such.
}

impl GemtextWidget {
    pub fn ui(&mut self, ui: &mut Ui) -> Response {
        // Assuming we're in a top-down layout, because that's all that really makes sense:
        let mut layout = *ui.layout();
        layout.cross_justify = self.justify;

        ui.with_layout(layout, |ui| self.render(ui));

        Response {
            link_clicked: self.link_clicked.take(),
        }
    }

    /// Clear all state gathered by our render.

    fn render(&mut self, ui: &mut Ui) {
        let mut line_num: u32 = 0;
        for block in &self.blocks {
            line_num += 1;
            match block {
                Block::Heading { level, text } => {
                    let is_title = line_num == 1 && *level == 1;
                    let style = if is_title { Style::Title() } else { Style::heading(*level) };
                    let rt = RichText::new(text).text_style(style).strong();
                    if is_title {
                        let layout: Layout = Layout::top_down(Align::Center);
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
                    let size = vec2(ui.available_width(), ui.text_style_height(&TextStyle::Body));
                    let layout = Layout::left_to_right(Align::BOTTOM).with_main_wrap(true);
                    ui.allocate_ui_with_layout(size, layout, |ui| {
                        ui.label(" * "); // TODO
                        ui.label(text);
                    });
                },
                Block::BlockQuote { lines } => {
                    block_quote(ui, lines);
                },
                Block::CodeFence { meta, lines } => {
                    for line in lines {
                        ui.monospace(line);
                    }
                },
                Block::Link { url, text } => {
                    let visible = if text.is_empty() { url } else { text };
                    let link = Link::new(visible);
                    let response = ui.add(link);
                    if response.clicked() {
                        self.link_clicked = Some(url.clone());
                    }
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
            // Draw border right between these?:
            .outer_margin(left_margin)
            .inner_margin(left_margin);
        frame.show(ui, |ui| {
            for line in lines {
                if let Block::Text(line) = line {
                    ui.label(line);
                }
            }
        });

    });
    let rect = response.response.rect;
    let left_bump = vec2(left_margin.left, 0.0);
    ui.painter().line_segment(
        [rect.left_top() +left_bump, rect.left_bottom() + left_bump],
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

    /// The first H1 in a Gemtext is the page Title:
    pub fn Title() -> TextStyle { Self::named("Title") }

    fn named(name: &str) -> TextStyle { TextStyle::Name(name.into()) }

    pub fn config(ctx: &egui::Context) {
        use egui::FontFamily::Proportional;
        let body_size = ctx.style().text_styles.get(&TextStyle::Body).expect("TextStyle::Body should always be present").size;
        ctx.all_styles_mut(|style| {
            style.text_styles.entry(Self::Title()).or_insert(FontId::new(body_size * 2.0, Proportional));
            style.text_styles.entry(Self::h1()).or_insert(FontId::new(body_size * 2.0, Proportional));
            style.text_styles.entry(Self::h2()).or_insert(FontId::new(body_size * 1.5, Proportional));
            style.text_styles.entry(Self::h3()).or_insert(FontId::new(body_size * 1.2, Proportional));            
        });
    }
}