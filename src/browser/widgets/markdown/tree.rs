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

                        Tag::Emphasis => {
                            let inline = Inline::Styled {
                                style:  Style::Italics,
                                parts: self.parse_inline(&|end| end == TagEnd::Emphasis)
                            };
                            blocks.push_inline(inline);
                        },
                        Tag::Strong => {
                            let inline = Inline::Styled {
                                style:  Style::Bold,
                                parts: self.parse_inline(&|end| end == TagEnd::Strong)
                            };
                            blocks.push_inline(inline);
                        },
                        Tag::Link { link_type, dest_url, title, id } => {
                            for inline in self.parse_link(link_type, dest_url, title, id) {
                                blocks.push_inline(inline);
                            }
                        },
                        Tag::Image { id, dest_url, link_type: _, title } => {
                            blocks.push_inline(self.parse_image(dest_url, title, id));
                        },


                        tag @ Tag::Strikethrough
                        | tag @ Tag::Superscript
                        | tag @ Tag::Subscript
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
                },

                SoftBreak => {
                    // TODO: Check whether we need this space. (Collapse spaces)
                    blocks.push_inline(Inline::Text(" ".into()))
                },
                HardBreak => {
                    // TODO: Check whether we need this space. (Collapse spaces)
                    blocks.push_inline(Inline::Text("\n".into()))
                },

                Code(mono) => {
                    blocks.push_inline(Inline::Code(mono.into()));
                },

                item @ End(_)
                | item @ Code(_)
                | item @ InlineMath(_)
                | item @ DisplayMath(_)
                | item @ Html(_)
                | item @ InlineHtml(_)
                | item @ FootnoteReference(_)
                | item @ TaskListMarker(_) => {
                    let msg = format!("(Unimplemented top-level item: {item:?})");
                    blocks.push_inline(msg.into());
                },
            }
        }
        
        blocks
    }
    
    fn parse_p(&mut self) -> Block {
        let parts: Vec<Inline> = self.parse_inline(&|tag| tag == TagEnd::Paragraph);
        Block::P{ parts }
    }

    fn parse_list_item(&mut self) -> Block {
        let blocks = self.parse_blocks_until(|end| matches!(end, TagEnd::Item));

        Block::ListItem {
            blocks
        }
    }

    // Reusable inline parser.
    fn parse_inline(&mut self, end_condition: &dyn Fn(TagEnd) -> bool) -> Vec<Inline> {
        // Re-use the block-level parsing:
        let blocks = self.parse_blocks_until(end_condition);

        // But we expect to only get inline items in this context, so check & extract those:
        let mut inlines: Vec<Inline> = vec![];
        for block in blocks {
            match block {
                Block::PseudoP { parts } => {
                    inlines.extend(parts);
                },
                block => {
                    inlines.push(format!("(Unexpected Inline Block: {block:?})").into());
                }
            }
        }

        inlines
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
    
    fn parse_link(
        &mut self, 
        _link_type: pulldown_cmark::LinkType,
        dest_url: pulldown_cmark::CowStr<'_>,
        _title: pulldown_cmark::CowStr<'_>,
        _id: pulldown_cmark::CowStr<'_>
    ) -> Vec<Inline> {
        let mut out: Vec<Inline> = vec![];

        let parts = self.parse_inline(&|end| end == TagEnd::Link);

        // An HTML link can span multiple other elements, ex:
        // <a href="foo">Some text <img src="bar"/> Some more text</a>
        // But that complicates rendering. We'll just duplicate the link across all inner elements.

        for part in parts {
            out.push(match part {
                Inline::Text(text) => Inline::Link(Link{
                    text,
                    href: dest_url.clone().into(),
                }),

                inner @ Inline::Code(_)
                | inner @ Inline::Styled { .. } 
                => {
                    // TODO: I don't believe egui supports styled links.
                    let text = inner.extract_text();
                    Inline::Link(Link{
                        text,
                        href: dest_url.clone().into(),
                    })
                },
                Inline::Image(image @ Image{ .. }) => {
                    Inline::LinkedImage{
                        image: image.clone(),
                        link: Link { text: "".into(), href: dest_url.clone().into() }
                    }
                }
                inline @ Inline::LinkedImage { .. }
                | inline @ Inline::Link { .. } => Inline::Link(Link{
                    text: format!("Unexpected within link: {inline:?}").into(),
                    href: dest_url.clone().into(),
                }),
            });
        }
        out
    }
    
    fn parse_image(&mut self, dest_url: pulldown_cmark::CowStr<'_>, title: pulldown_cmark::CowStr<'_>, id: pulldown_cmark::CowStr<'_>) -> Inline {
        let parts = self.parse_inline(&|end| end == TagEnd::Image);
        let mut alt: String = if parts.is_empty() {
            "".into()
        } else if parts.len() > 1 {
            format!("(Error: Too many inline elements: {parts:?})")
        } else {
            let part = parts.into_iter().next().expect("Exactly 1 element");
            let text = match part {
                Inline::Text(text) => text,
                inline => { format!("(Error: Unexpected inline element in image: {inline:?})") }
            };
            text
        };

        if alt.is_empty() {
            // Given ![][foo], fall back to foo as the alt text.
            alt = id.into();
        }
        if alt.is_empty() {
            // TODO: Trim this:
            alt = dest_url.clone().into();    
        }

        Inline::Image(Image{ 
            src: dest_url.into(),
            alt,
            title: title.into()
        })
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
    Text(String),
    Code(String),
    Link(Link),
    
    /// Just a normal Markdown(/HTML) image. We make these links so you can browse to the image itself to view it.
    Image(Image),

    /// An image that had a link in the original markdown, so we need to display 2 links.
    /// Note: link.text will be empty.
    LinkedImage { link: Link, image: Image },

    Styled {
        style: Style,
        parts: Vec<Inline>
    },

}
impl Inline {
    fn extract_text(&self) -> String {
        match self {
            Inline::Text(text) => text.into(),
            Inline::Code(text) => text.into(),
            Inline::Link(Link{ text, href: _ }) => text.into(),
            Inline::Image(Image{ src, alt: _, title: _ }) => src.into(),
            Inline::LinkedImage { image, link: _ } => image.src.clone(),
            Inline::Styled { parts, style: _} => {
                parts.into_iter()
                    .map(|part| part.extract_text())
                    .collect::<Vec<_>>()
                    .join(" ")
            },
        }
    }
}

/// A simple text link.
#[derive(Clone, Debug)]
pub struct Link {
    pub href: String,
    pub text: String,
}

#[derive(Clone, Debug)]
pub struct Image {
    pub src: String,
    /// Displayed instead of the image. (egemi won't display images inline)
    pub alt: String,
    /// Displayed on hover.
    pub title: String, 
}

#[derive(Debug)]
pub enum Style {
    Bold,
    Italics,
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