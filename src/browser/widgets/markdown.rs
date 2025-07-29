use eframe::egui::{self, Align, Layout, RichText, Ui, Vec2};
use log::debug;
use pulldown_cmark::{Tag, TagEnd};

use crate::{browser::{network::SCow, parsers::html::to_md, widgets::DocWidget}, gemtext_widget::Style};

use super::DocumentResponse;

#[derive(Debug)]
pub struct MarkdownWidget {
    // Should hard-code to false until this bug is fixed:
    // https://github.com/emilk/egui/issues/1272
    justify: bool,

    parsed: Parsed,
    link_clicked: Option<String>,
}

impl MarkdownWidget {
    pub fn for_html(html: &str) -> Self {
        let md = to_md(html);
        Self::for_md(&md)
    }

    pub fn for_md(md: &str) -> Self {
        Self {
            justify: false,
            parsed: Parser::from_md(md),
            link_clicked: None,
        }
    }
}

impl MarkdownWidget {
    fn render(&mut self, ui: &mut Ui) {
        let mut block_num: u64 = 0;
        for block in &self.parsed.blocks {
            block_num += 1;
            match block {
                Block::Heading { level, text } => {
                    let is_title = block_num == 1 && *level == 1;
                    let style = if is_title { Style::title() } else { Style::heading(*level) };
                    let rt = RichText::new(text).text_style(style).strong();
                    if is_title {
                        ui.vertical_centered(|ui| {
                            ui.label(rt);
                        });
                    } else {
                        ui.label(rt);
                    }
                    self.line_spacing(ui);
                },
                Block::Pre { text } => {
                    // ui.monospace(line);
                    let rt = RichText::new(text).text_style(Style::mono());
                    ui.label(rt);
                },
                Block::BlockQuote { blocks } => {
                    // TODO
                },
                Block::P { parts } => {
                    ui.horizontal_wrapped(|ui| {
                        let response = self.render_inline(ui, parts);
                        if let Some(link_clicked) = response {
                            // self.link_clicked = Some(link_clicked);
                        }
                    });
                    self.line_spacing(ui);
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

/// pulldown-commonmark gives a parser as an iterator, but no way to serialize the parsed document.
/// Which means we would have to re-parse it with every render to screen. Booo.
/// Instead, let's parse the parts of Markdown we want to support into a flat format, which we can quickly (re)render.
/// 
#[derive(Debug)]
struct Parsed {
    // TODO: title: Option<String>

    blocks: Vec<Block>
}

struct Parser {
    // may one day have options for how to parse text? Or do we want to put the options on the rendering side?)
}

impl Parser {
    fn from_html(html: &str) -> Parsed {
        let md = to_md(html);
        Parser::from_md(&md)
    }

    fn from_md(md: &str) -> Parsed {
        let mut parser = pulldown_cmark::Parser::new(&md);
        let mut blocks: Vec<Block> = vec![];

        use pulldown_cmark::Event::*;
        while let Some(event) = parser.next() {
            match event {
                Start(tag) => {
                    match tag {
                        Tag::Paragraph => {
                            blocks.push(Self::parse_p(&mut parser));
                        },

                        tag @ Tag::Heading { level, ..} => {
                            blocks.push(Self::parse_heading(&mut parser, level));
                        },
                        tag @ Tag::BlockQuote(block_quote_kind) => {
                            eprintln!("TODO: {tag:?}");
                        },
                        tag @ Tag::CodeBlock(_) => {
                            eprintln!("TODO: {tag:?}");
                        },
                        tag @ Tag::HtmlBlock => {
                            eprintln!("TODO: {tag:?}");
                        },
                        tag @ Tag::List(_) => {
                            eprintln!("TODO: {tag:?}");
                        },
                        tag @ Tag::Item => {
                            eprintln!("TODO: {tag:?}");
                        },

                        tag @ Tag::DefinitionList
                        | tag @ Tag::DefinitionListTitle
                        | tag @ Tag::DefinitionListDefinition
                        | tag @ Tag::Table(_)
                        | tag @ Tag::TableHead
                        | tag @ Tag::TableRow
                        | tag @ Tag::TableCell => {
                            // We haven't enabled these.
                            blocks.push(format!("Unexpected tag: {tag:?}").into());
                        },

                        tag @ Tag::Emphasis
                        | tag @ Tag::Strong
                        | tag @ Tag::Strikethrough
                        | tag @ Tag::Superscript
                        | tag @ Tag::Subscript
                        | tag @ Tag::Link { .. }
                        | tag @ Tag::Image { .. }
                        | tag @ Tag::MetadataBlock(_) => {
                            eprintln!("TODO: {tag:?}");
                        },

                        tag @ Tag::FootnoteDefinition(_) => {
                            blocks.push(format!("Unsupported tag: {tag:?}").into());
                        },
                    }
                },
                item @ Text(_)
                | item @ End(_)
                | item @ Code(_)
                | item @ InlineMath(_)
                | item @ DisplayMath(_)
                | item @ Html(_)
                | item @ InlineHtml(_)
                | item @ FootnoteReference(_)
                | item @ SoftBreak
                | item @ HardBreak
                | item @ Rule
                | item @ TaskListMarker(_) => {
                    let msg = format!("Unimplemented top-level item: {item:?}");
                    blocks.push(msg.into());
                },
            }
        }

        Parsed { blocks }
    }
    
    fn parse_p(parser: &mut pulldown_cmark::Parser<'_>) -> Block {
        let mut parts: Vec<Inline> = vec![];

        while let Some(event) = parser.next() {
            use pulldown_cmark::Event::*;
            match event {
                End(tag) if tag == TagEnd::Paragraph => {
                    // HTML paragraphs can't be nested.
                    // Any end paragraph means we're done parsing this paragraph.
                    break;
                }
                Start(tag) => {
                    eprintln!("TODO: Start tag: {tag:?}")
                },
                End(tag_end) => {
                    eprintln!("TODO: End tag: {tag_end:?}")
                },
                Text(cow_str) => {
                    parts.push(Inline::Text(cow_str.into()))
                },
                Code(cow_str) => {
                    // TODO: Inline code support.
                    parts.push(Inline::Text(cow_str.into()))
                },

                // None of these are really supported but we'll just display them rather than hiding them:
                InlineMath(cow_str)
                | DisplayMath(cow_str)
                | Html(cow_str)
                | InlineHtml(cow_str)
                | FootnoteReference(cow_str) => {
                    parts.push(Inline::Text(cow_str.into()))
                },


                SoftBreak => {
                    parts.push(" ".to_string().into());
                },
                HardBreak => parts.push(Inline::Br),
                tag @ Rule => {
                    // Shouldn't really be part of a paragraph?
                    let msg = format!("Unexpected tag: {tag:?}");
                    parts.push(msg.into());
                },
                tlm @ TaskListMarker(checked) => {
                    // Shouldn't ever happen since we disabled it, but be verbose if it does:
                    let text = format!("{tlm:?}");
                    parts.push(Inline::Text(text));
                },
            }
        }

        Block::P{ parts }
    }
    
    fn parse_heading(parser: &mut pulldown_cmark::Parser<'_>, level: pulldown_cmark::HeadingLevel) -> Block {
        use pulldown_cmark::HeadingLevel::*;
        let level = match level {
            H1 => 1,
            H2 => 2,
            H3 => 3,
            H4 => 4,
            H5 => 5,
            H6 => 6,
        };
        let mut text = String::new();

        while let Some(event) = parser.next() {
            use pulldown_cmark::Event::*;
            match event {
                End(TagEnd::Heading(_)) => break,
                Text(cow_str) => text.push_str(&cow_str),
                
                event => {
                    debug!("Skipping unsupported heading event: {event:?}");
                }
            }
        }

        Block::Heading { level, text }
    }
}

/// A parsed, top-level block of markdown.
#[derive(Debug)]
enum Block {
    Heading{ level: u8, text: String },
    Pre{ text: String },
    BlockQuote {
        blocks: Vec<Block>
    },
    P{ 
        parts: Vec<Inline>
    },
}

/// Mostly usedu for debugging unexpected Markdown formats.
impl From<String> for Block {
    fn from(value: String) -> Self {
        Block::P { parts: vec![
            Inline::Text(value)
        ] }
    }
}

#[derive(Debug)]
enum Inline {
    Br,
    Text(String),
    Link{ text: String, href: String }
}

// Mostly for debug errors.
impl From<String> for Inline {
    fn from(value: String) -> Self {
        Inline::Text(value)
    }
}