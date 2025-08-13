use std::sync::Arc;

use eframe::{egui::{self, Align, Color32, Frame, Layout, Link, RichText, TextStyle, Ui, UiBuilder, Vec2}, epaint::MarginF32};
use log::debug;
use pulldown_cmark::{Tag, TagEnd};

use crate::{browser::{network::SCow, parsers::html::to_md, widgets::{markdown::tree::{Block, Image, Inline}, DocWidget}}, gemtext_widget::Style};

use super::DocumentResponse;
mod tree;
mod tree_test;

#[derive(Debug)]
pub struct MarkdownWidget {
    // Should hard-code to false until this bug is fixed:
    // https://github.com/emilk/egui/issues/1272
    justify: bool,

    parsed_blocks: Arc<Vec<tree::Block>>,
    link_clicked: Option<String>,

    text_bold: bool,
    text_italics: bool,
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
            parsed_blocks: Arc::new(parsed.blocks),
            link_clicked: None,
            text_bold: false,
            text_italics: false,
        }
    }
}

impl MarkdownWidget {
    fn render(&mut self, ui: &mut Ui) {
        let blocks = Arc::clone(&self.parsed_blocks);
        self.render_blocks(ui, &blocks);
        ui.label("");

        // return click events
    }

    fn render_blocks(&mut self, ui: &mut Ui, blocks: &[Block]) {
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

    fn render_block(&mut self, ui: &mut Ui, block: &Block) {
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
    
    fn render_list(&mut self, ui: &mut Ui, start_num: Option<u64>, blocks: &[Block]) {
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

    fn render_inline(&mut self, ui: &mut Ui, parts: &[Inline]){
        for part in parts {
            match part {
                Inline::Text(text) => { 
                    let mut text = RichText::new(text);
                    if self.text_italics {
                        text = text.italics();
                    }
                    if self.text_bold {
                        text = text.strong();
                    }
        
                    ui.label(text); 
                },
                Inline::Code(text) => {
                    ui.monospace(text);
                }
                Inline::Link(tree::Link{ text, href }) => {
                    let link = egui::Link::new(text);
                    let response = ui.add(link);
                    if response.clicked() {
                        self.link_clicked = Some(href.clone());
                    }
                    response.on_hover_ui(|ui| {
                        ui.monospace(href);
                    });
                },
                Inline::Styled { style, parts } => {
                    use tree::Style::*;
                    match style {
                        Bold => {
                            self.text_bold = true;
                            self.render_inline(ui, &parts);
                            self.text_bold = false;
                        },
                        Italics => {
                            self.text_italics = true;
                            self.render_inline(ui, &parts);
                            self.text_italics = false;
                        },
                    };
                },
                Inline::Image(Image { src, title, alt }) => {
                    // We render this like a link, but surrounded w/ Markdown image syntax.
                    // In the future we can add options for different ways to display/distinguish image links.
                    let response = ui.link(format!("![{alt}]"));
                    if response.clicked() {
                        self.link_clicked = Some(src.clone())
                    }
                    response.on_hover_ui(|ui| {
                        ui.monospace(src);
                        if !title.is_empty() {
                            ui.label(title);
                        }
                    });
                },
                Inline::LinkedImage { link, image } => {
                    let Image{alt, src, title} = image;
                    // Same as above, but we append an [href] link too:
                    let response = ui.link(format!("![{alt}]"));
                    if response.clicked() {
                        self.link_clicked = Some(src.clone());
                    }
                    response.on_hover_ui(|ui| {
                        ui.monospace(src);
                        if !title.is_empty() {
                            ui.label(title);
                        }
                    });

                    if link.href != image.src {
                        let r2 = ui.link("[href]");
                        if r2.clicked() {
                            self.link_clicked = Some(link.href.clone());
                        }
                        r2.on_hover_ui(|ui| {
                            ui.monospace(&link.href);
                        });
                    }
                }
            }
        }
    }

    fn render_bq(&mut self, ui: &mut Ui, blocks: &[Block]) {
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
            link_clicked: self.link_clicked.take(),
        }
    }
}

