//! Utilities for parsing gemtext.

use std::sync::LazyLock;

use regex::Regex;

/// A parsed chunk of Gemtext.
/// Usually, each block is a single line.
/// However, code fences and blockquotes are grouped together.
#[derive(Debug)]
pub enum Block {
    /// A heading, preceeded by #
    Heading{
        /// Gemtext supports levels 1, 2, & 3 but this parser will parse an arbitrary number < 256.
        level: u8,
        text: String,
    },

    /// A plain-text line.
    Text(String),

    /// Unordered list item.
    ListItem {
        text: String
    },

    /// One or more quoted lines.
    /// As documented, Gemtext format doesn't allow anything other than plaintext quoted lines, but the parser 
    /// may in the future support quoting gemtext format.
    BlockQuote {
        lines: Vec<Block>
    },

    /// Blocks of code surrounted by triple-backticks.
    /// A missing end block will leave the rest of a documented as code.
    CodeFence {
        /// Optional metadata that appears after the opening triple-ticks.
        meta: String,
        lines: Vec<String>,
    },

    /// A link. Starts with `=> `
    Link {
        url: String,
        text: String,
    },
}

/// Options for the parser. We may one day have these. 
#[derive(Default, Debug)]
pub struct Options {
    strict: bool,
}

const CODE_GUARD: &str = "```";
const BLOCK_QUOTE: &str = ">";

impl Options {
    pub fn parse(&self, value: &str) -> Result<Vec<Block>, String> {
        let mut code: Option<CodeFence> = None;
        let mut quote: Option<Vec<String>> = None;
        let mut blocks = Vec::new();
        for line in value.lines() {
            if let Some(meta) = line.strip_prefix(CODE_GUARD) {
                let meta = meta.trim();
                if let Some(existing) = code.take() {
                    if !meta.is_empty() && self.strict {
                        return Err(format!("Found end code guard with meta: {meta}"))
                    }
                    blocks.push(Block::CodeFence{
                        meta: existing.meta,
                        lines: existing.lines
                    });
                    continue;
                }
                // else: starting new block:
                code = Some(CodeFence{meta: String::from(meta), lines: Vec::new()});
                continue;
            }
            if let Some(code) = &mut code {
                let mut line = line.to_string();
                if line.starts_with(" ") {
                    // See: https://github.com/emilk/egui/issues/1272
                    line = line.replacen(" ", "\u{a0}", 5);
                }
                code.lines.push(line.into());
                continue;
            }
            if let Some(text) = line.strip_prefix(BLOCK_QUOTE) {
                let text = text.trim().to_string();
                if let Some(quote) = &mut quote {
                    quote.push(text)
                } else {
                    quote = Some(vec![text]);
                }
                continue
            }
            if let Some(quote) = quote.take() {
                blocks.push(Block::BlockQuote{
                    lines: quote.into_iter().map(|it| Block::Text(it)).collect()
                })
            }

            if let Some(HeaderLine{level, text}) = HeaderLine::parse(line) {
                blocks.push(Block::Heading { level, text });
                continue;
            }

            if let Some(LinkLine{url, text}) = LinkLine::parse(line) {
                blocks.push(Block::Link { url, text });
                continue;
            }
            
            if let Some(ListItem{text}) = ListItem::parse(line) {
                blocks.push(Block::ListItem{text});
                continue;
            }

            blocks.push(Block::Text(line.into()));

        } // lines

        // Don't forget unclosed blocks!
        if let Some(CodeFence{meta, lines}) = code {
            blocks.push(Block::CodeFence { meta, lines })
        }
        if let Some(quote) = quote {
            blocks.push(Block::BlockQuote { 
                lines: quote.into_iter().map(|it| Block::Text(it)).collect()
             })
        }

        Ok(blocks)
    }
}

struct CodeFence {
    meta: String,
    lines: Vec<String>,
}

struct HeaderLine { 
    level: u8,
    text: String,
}

impl <'a> HeaderLine {
    fn parse(value: &'a str) -> Option<HeaderLine> {
        static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(
            "^(?P<hashes>#+)([ ]+)(?P<heading>.*)$"
        ).unwrap());

        let caps = RE.captures(value);
        let Some(caps) = caps else {
            return None;
        };
        let hashes = &caps["hashes"];
        let heading = &caps["heading"];
        return Some(HeaderLine {
            level: hashes.len() as u8,
            text: heading.into()
        });
    }
}

struct LinkLine {
    url: String,
    text: String,
}

impl LinkLine {
    fn parse(value: &str) -> Option<Self> {
        static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(
            r#"^=>\s+(?P<url>\S+)(\s+(?P<text>\S.*?))?\s*$"#
        ).unwrap());

        let Some(caps) = RE.captures(value) else {
            return None;
        };

        let url = &caps["url"];
        let text = caps.name("text").map(|it| it.as_str()).unwrap_or("");
        Some(Self {
            url: url.into(),
            text: text.into(),
        })
    }
}

struct ListItem {
    text: String
}

impl ListItem {
    fn parse(value: &str) -> Option<Self> {
        static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(
            r#"^\s?\*\s(?P<text>.+?)\s*$"#
        ).unwrap());

        let Some(caps) = RE.captures(value) else {
            return None;
        };

        let text = caps.name("text").unwrap().as_str().to_owned();
        return Some(Self{text})
    }

}