use eframe::{egui::{self, Align, Color32, Frame, Layout, RichText, TextStyle, Ui, UiBuilder, Vec2}, epaint::MarginF32};
use log::debug;
use pulldown_cmark::{Tag, TagEnd};

use crate::{browser::{network::SCow, parsers::html::to_md, widgets::{markdown::tree::{Block, Inline}, DocWidget}}, gemtext_widget::Style};

use super::DocumentResponse;
mod tree;

#[derive(Debug)]
pub struct MarkdownWidget {
    // Should hard-code to false until this bug is fixed:
    // https://github.com/emilk/egui/issues/1272
    justify: bool,

    parsed: tree::Parsed,
    link_clicked: Option<String>,
}

impl MarkdownWidget {
    pub fn for_html(html: &str) -> Self {
        let md = to_md(html);
        Self::for_md(&md)
    }

    pub fn for_md(md: &str) -> Self {
        let parsed = tree::Parser::from_md(md);
        debug!("Parsed markdown: {parsed:#?}");
        Self {
            justify: false,
            parsed,
            link_clicked: None,
        }
    }
    

}

impl MarkdownWidget {
    fn render(&mut self, ui: &mut Ui) {
        self.render_blocks(ui, &self.parsed.blocks);
        ui.label("");
    }

    fn render_blocks(&self, ui: &mut Ui, blocks: &[Block]) {
        let last_block_num = blocks.len();
        let mut block_num = 0;
        for block in blocks {
            block_num += 1;
            let last_block = block_num == last_block_num;
            self.render_block(ui, block);
            
            let is_pseudo = matches!(block, Block::PseudoP { .. });
            if !last_block && !is_pseudo { self.line_spacing(ui); }
        }
    }

    fn render_block(&self, ui: &mut Ui, block: &Block) {
        match block {
            Block::Heading { level, text } => {
                let style = Style::heading(*level);
                let rt = RichText::new(text).text_style(style).strong();
                ui.label(rt);
            },
            Block::CodeBlock { text, .. } => {
                let rt = RichText::new(text).text_style(Style::mono());
                ui.label(rt);
            },
            Block::BlockQuote { blocks } => {
                self.render_bq(ui, blocks);
            },
            Block::P { parts } | Block::PseudoP { parts } => {
                ui.horizontal_wrapped(|ui| {
                    let response = self.render_inline(ui, parts);
                    if let Some(link_clicked) = response {
                        // self.link_clicked = Some(link_clicked);
                    }
                });
            },
            Block::List { start_num, blocks } => {
                self.render_list(ui, start_num.clone(), blocks);
            },
            Block::ListItem { .. } => {
                // ListItems should always appear directly in a List, right?
                ui.colored_label(Color32::from_rgb(255, 0, 0), "Error: Unexpected ListItem outside of List");
            },
            Block::Hr => {
                ui.separator();
            }
        }
    }
    
    fn render_list(&self, ui: &mut Ui, start_num: Option<u64>, blocks: &[Block]) {
        let mut start_num = start_num;
        for block in blocks {
            match block {
                Block::List { start_num, blocks } => {
                    // TODO: Adjust indentation.
                    ui.indent("list", |ui| {
                        self.render_list(ui, start_num.clone(), blocks);
                    });
                },
                Block::ListItem { blocks } => {
                    let bullet = if let Some(num) = &mut start_num {
                        let out = format!("{num}. ");
                        *num += 1;
                        out
                    } else {
                        " • ".to_string()
                    };
                    ui.horizontal_top(|ui| {
                        ui.label(bullet);
                        ui.vertical(|ui| {
                            self.render_blocks(ui, blocks);
                        })
                    });
                },
                block => {
                    // Shouldn't happen? But if so, just render:
                    ui.label(format!("Unexpected block in List: {block:?}"));
                }
            }
        }
    }

    fn line_spacing(&self, ui: &mut Ui) {
        // Markdown paragraphs and H1s usually have implicit padding between them. We can just add a newline.
        ui.label("");
    }

    fn render_inline(&self, ui: &mut Ui, parts: &[Inline]) -> Option<String> {
        let mut link_clicked = None;
        for part in parts {
            match part {
                Inline::Br => { ui.label("\n"); },
                Inline::Text(text) => { ui.label(text); },
                Inline::Link { text, href } => {
                    let link = egui::Link::new(text);
                    let response = ui.add(link);
                    if response.clicked() {
                        link_clicked = Some(href.clone());
                    }
                    response.on_hover_ui(|ui| {
                        ui.monospace(href);
                    });
                },
            }
        }

        link_clicked
    }

    fn render_bq(&self, ui: &mut Ui, blocks: &[Block]) {
        let builder = UiBuilder::new();
        let row_height = ui.text_style_height(&TextStyle::Body);
        let left_margin = MarginF32{ left: row_height / 2.0, ..Default::default() };
        let response = ui.scope_builder(builder, |ui| {
            let frame = Frame::new()
                .outer_margin(left_margin);
            frame.show(ui, |ui| {
                self.render_blocks(ui, blocks);
            });

        });
        let rect = response.response.rect;
        ui.painter().line_segment(
            [rect.left_top(), rect.left_bottom()],
            (1.0, ui.visuals().weak_text_color()),
        );
    }
}

impl DocWidget for MarkdownWidget {
    fn ui(&mut self, ui: &mut Ui) -> DocumentResponse {
        // Unlike Gemtext, markdown can have inline styling and links,
        // so we'll layout text horizontally and manually add rows when we get to block elements.
        // let layout = Layout::left_to_right(Align::TOP).with_main_justify(self.justify);

        // Assuming we're in a top-down layout, because that's all that really makes sense:
        let mut layout = *ui.layout();
        layout.cross_justify = self.justify;

        ui.with_layout(layout, |ui| {
            // TODO: We may need to explicitly add whitespace between adjacent text items if markdown doesn't.
            ui.spacing_mut().item_spacing = Vec2::ZERO;

            self.render(ui)
        });
        DocumentResponse {
            link_clicked: None,
        }
    }
}

