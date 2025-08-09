use log::debug;
use pulldown_cmark::{CodeBlockKind, Parser as CmParser, Tag, TagEnd, TextMergeStream};

use crate::browser::parsers::html::to_md;

/// pulldown-commonmark gives a parser as an iterator, but no way to serialize the parsed document.
/// Which means we would have to re-parse it with every render to screen. Booo.
/// Instead, let's parse the parts of Markdown we want to support into a data structure, which we can quickly (re)render.
/// 
#[derive(Debug)]
pub struct Parsed {
    // TODO: title: Option<String>

    pub blocks: Vec<Block>
}

pub struct Parser<'a> {
    inner: TextMergeStream<'a, CmParser<'a>>
}

impl <'a> Parser<'a> {
    pub fn from_html(html: &str) -> Parsed {
        let md = to_md(html);
        Parser::from_md(&md)
    }

    pub fn from_md(md: &str) -> Parsed {
        let mut parser = Parser {
            inner: TextMergeStream::new(CmParser::new(&md))
        };
        parser.parse_all()
    }

    fn parse_all(&mut self) -> Parsed {
        Parsed {
            blocks: self.parse_blocks_until(|_| false)
        }
    }

    /// Reusable top-level parser that can recurse.
    fn parse_blocks_until(&mut self, matches: impl Fn(TagEnd) -> bool) -> Vec<Block> {
        // TODO: Depth check to prevent stack overflows.

        let mut blocks: Vec<Block> = vec![];
    
        use pulldown_cmark::Event::*;
        while let Some(event) = self.inner.next() {
            match event {
                End(tag) if matches(tag) => { return blocks; },
                Start(tag) => {
                    match tag {
                        Tag::Paragraph => {
                            blocks.push(self.parse_p());
                        },
                        Tag::Heading { level, ..} => {
                            blocks.push(self.parse_heading(level));
                        },
                        Tag::BlockQuote(_) => {
                            blocks.push(self.parse_bq());
                        },
                        Tag::CodeBlock(kind) => {
                            blocks.push(self.parse_code(kind.into_static()))
                        },
                        tag @ Tag::HtmlBlock => {
                            blocks.push(format!("TODO: Start Tag {tag:?}").into());
                        },
                        Tag::List(start_info) => {
                            blocks.push(self.parse_list(start_info));
                        },
                        Tag::Item => {
                            blocks.push(self.parse_list_item());
                        },

                        tag @ Tag::DefinitionList
                        | tag @ Tag::DefinitionListTitle
                        | tag @ Tag::DefinitionListDefinition
                        | tag @ Tag::FootnoteDefinition(_)
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
                    }
                },
                Text(text) => {
                    blocks.push_inline(Inline::Text(text.into()))
                },
                Rule => {
                    blocks.push(Block::Hr);
                }

                item @ End(_)
                | item @ Code(_)
                | item @ InlineMath(_)
                | item @ DisplayMath(_)
                | item @ Html(_)
                | item @ InlineHtml(_)
                | item @ FootnoteReference(_)
                | item @ SoftBreak
                | item @ HardBreak
                | item @ TaskListMarker(_) => {
                    let msg = format!("Unimplemented top-level item: {item:?}");
                    blocks.push(msg.into());
                },
            }
        }
        
        blocks
    }
    
    fn parse_p(&mut self) -> Block {
        let parts: Vec<Inline> = self.parse_inline(|tag| tag == TagEnd::Paragraph);
        Block::P{ parts }
    }

    fn parse_list_item(&mut self) -> Block {
        let blocks = self.parse_blocks_until(|end| matches!(end, TagEnd::Item));

        Block::ListItem {
            blocks
        }
    }



    // Reusable inline parser.
    fn parse_inline(&mut self, end_condition: impl Fn(TagEnd) -> bool) -> Vec<Inline> {
        let mut parts: Vec<Inline> = vec![];

        while let Some(event) = self.inner.next() {
            use pulldown_cmark::Event::*;
            match event {
                End(tag) if end_condition(tag) => { break; }
                event @ Start(_) => {
                    parts.push(format!("(TODO: {event:?})").into());
                },
                event @ End(_) => {
                    parts.push(format!("(TODO: {event:?})").into());
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
                    // TODO: Check previous/next parts and don't add a space if it's unnecssary. (Collapse spaces)
                    parts.push(" ".to_string().into());
                },
                HardBreak => parts.push(Inline::Br),
                tag @ Rule => {
                    // Shouldn't really be part of a paragraph?
                    let msg = format!("Unexpected tag: {tag:?}");
                    parts.push(msg.into());
                },
                tlm @ TaskListMarker(_) => {
                    // Shouldn't ever happen since we disabled it, but be verbose if it does:
                    let text = format!("{tlm:?}");
                    parts.push(Inline::Text(text));
                },
            }
        }

        parts
    }

    fn parse_list(&mut self, start_num: Option<u64>) -> Block {
        let blocks = self.parse_blocks_until(|tag| matches!(tag, TagEnd::List(_)));

        Block::List {
            start_num,
            blocks,
        }
    }
    
    fn parse_heading(&mut self, level: pulldown_cmark::HeadingLevel) -> Block {
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

        while let Some(event) = self.inner.next() {
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
    
    fn parse_bq(&mut self) -> Block {

        let blocks = self.parse_blocks_until(|tag| matches!(tag, TagEnd::BlockQuote(_)));
        Block::BlockQuote { blocks }
    }

    fn parse_code(&mut self, kind: CodeBlockKind<'static>) -> Block {

        // Collect all text inside the code block.
        // Parser might break it up into multiple blocks as a side effect of parsing.
        let mut strings: Vec<String> = vec![];
        use pulldown_cmark::Event::*;
        while let Some(event) = self.inner.next() {
            match event {
                End(TagEnd::CodeBlock) => {
                    break;
                },
                Text(cow) => {
                    strings.push(cow.into());
                },
                tag => {
                    let msg = format!("\n\nERROR: Unexpected Markdown Event in code block: {tag:?}\n\n");
                    strings.push(msg);
                }
            }
        }
        
        Block::CodeBlock { 
            fenced: match kind {
                CodeBlockKind::Indented => None,
                CodeBlockKind::Fenced(cow_str) => Some(cow_str.into()),
            },
            text: strings.join(""),
        }
    }
}

/// A parsed, top-level block of markdown.
#[derive(Debug)]
pub enum Block {
    Heading{ level: u8, text: String },
    CodeBlock { 
        /// If fenced, this is set with the fenced metadata.
        fenced: Option<String>,
        text: String
    },
    BlockQuote {
        blocks: Vec<Block>
    },

    P{ 
        parts: Vec<Inline>
    },

    /// Note: This Paragraph representation differs from the markdown representation.
    /// Markdown is tightly coupled to HTML, so has some odd quirks. 
    /// For example, a <li> may contain inline text elements alongside block-level elements
    /// Like:  <li>Foo <ul>...</ul></li>
    /// Which is distinct from: <li><p>Foo</p><ul> in that the paragraph has an implicit 1em bottom margin.
    /// However, the inline text will still be **rendered as a block** (i.e. break normal text flow on top/bottom)
    /// So we need a way to group consecutive inline (optionally styled/linked) texts into a visual block without being a paragraph.
    PseudoP {
        parts: Vec<Inline>
    },

    /// Contains list items.
    List {
        start_num: Option<u64>,
        // Should contain only list `Item`s or other `List`s, but not checked.
        blocks: Vec<Block>
    },

    ListItem { 
        blocks: Vec<Block> 
    },
    Hr,
}

/// Mostly used for debugging unexpected Markdown formats.
impl From<String> for Block {
    fn from(value: String) -> Self {
        Block::P { parts: vec![
            Inline::Text(value)
        ] }
    }
}

#[derive(Debug)]
pub enum Inline {
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

trait PushInline {
    fn push_inline(&mut self, element: Inline);
}

impl PushInline for Vec<Block> {
    fn push_inline(&mut self, element: Inline) {
        if let Some(Block::PseudoP { parts }) = self.last_mut() {
            parts.push(element)
        } else {
            self.push(Block::PseudoP { parts: vec![ element ] } );
        }
    }
}