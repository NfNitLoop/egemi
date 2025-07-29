//! Very simplified (& lossy) parsing for HTML.

use std::{collections::VecDeque, sync::LazyLock};

use regex::{Regex, Replacer};
use tl::{Node, Parser, VDom};

use crate::browser::network::SCow;


/// Parses and flattens HTML.
/// 
/// HTML can have lots of nested data structures, like <div><div><span><article><etc>
/// But we're just parsing them to a flatter format suitable for displaying like Markdown or Gemtext.
pub struct FlatParser;


impl FlatParser {
    pub fn parse<'a>(&self, dom: & VDom<'a>) -> Vec<FlatNode> {
        let mut out: Vec<FlatNodeTemp> = vec![];

        let mut queue: VecDeque<tl::NodeHandle> = VecDeque::new();
        for node in dom.children() {
            queue.push_back(*node);
        }

        while let Some(handle) = queue.pop_front() {
            let Some(node) = handle.get(dom.parser()) else { continue };
            let tag = match node {
                Node::Tag(tag) => tag,
                Node::Comment(_bytes) => continue,
                Node::Raw(bytes) => {
                    let text = bytes.as_utf8_str().trim().to_owned();
                    if text.is_empty() { continue }
                    let text = text.to_owned();
                    out.push(text.into());
                    continue
                },
            };

            if skip_tag(tag) { continue }
            if let Some(node) = self.parse_tag(tag, dom.parser()) { 
                out.push(node.into());
                continue;
            }

            // Otherwise remove this node from the graph.
            // We just take its child nodes and add them in thos node's place in the DeQueue:
            let children: Vec<_> = tag.children().top().iter().cloned().collect();
            for node in children.into_iter().rev() {
                queue.push_front(node);
            }
        }

        collect_texts(out)
    }
    
    fn parse_tag(&self, tag: &tl::HTMLTag<'_>, parser: &Parser<'_>) -> Option<FlatNode> {
        let tag_name = tag.name().as_utf8_str().to_lowercase();
        if tag_name == "p" {
            return Some(self.parse_p(tag, parser));
        }

        let head_level = match tag_name.as_ref() {
            "h1" => Some(1),
            "h2" => Some(2),
            "h3" => Some(3),
            "h4" => Some(4),
            "h5" => Some(5),
            "h6" => Some(6),
            _ => None,
        };
        if let Some(level) = head_level {
            let text = html_to_plaintext(&tag.inner_text(parser));
            return Some(FlatNode::Heading(Heading { level, text: text.into() }))
        }

        println!("TODO: Parse tag: {tag_name}");
        None
    }
    
    fn parse_p(&self, tag: &tl::HTMLTag<'_>, parser: &Parser<'_>) -> FlatNode {
        // TODO: Join text parts together and collapse whitespace.
        let text: SCow = html_to_plaintext(&tag.inner_text(parser)).into();
        let parts = vec![
            ParaParts::Text(text)
        ];
        FlatNode::P(P { parts })
    }
}

/// Collapses whitespace (removing newlines), and parses some common HTML entities into their plaintext equivalent.
fn html_to_plaintext(value: &str) -> String {
    static WHITESPACE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"\s+"#).expect("regex"));
    let value = WHITESPACE.replace_all(value.trim(), " ").into_owned();
    let value = value
        // TODO: General purpose function for these?
        .replace("&amp;", "&")
        .replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">");

    value
}

fn collect_texts(input: Vec<FlatNodeTemp>) -> Vec<FlatNode> {
    let mut out: Vec<FlatNode> = vec![];
    for part in input {
        match part {
            FlatNodeTemp::FlatNode(node) => out.push(node),
            FlatNodeTemp::Text(txt) => {
                println!("TODO: handle text: {txt}");
                continue;
            }
        }
    }
    out
}

fn skip_tag(tag: &tl::HTMLTag<'_>) -> bool {
    let name = tag.name().as_utf8_str().to_lowercase();
    name == "script" || name == "style" || name == "head"
}


/// The top-level document will consist only of these.
#[derive(Debug)]
pub enum FlatNode {
    P(P),
    Heading(Heading),
    Pre(Pre),
    // TODO: <br>, <!-- comments -->, raw code blocks? maybe not.
}


/// Collect temporary data that can be converted into FlatNode.
enum FlatNodeTemp {
    // If we can immediately parse into a know tag, cool:
    FlatNode(FlatNode),

    // If we find random text outside of paragraphs, it might be a de facto paragraph, just in a <div> or something.
    Text(SCow),
}

impl From<String> for FlatNodeTemp {
    fn from(value: String) -> Self {
        FlatNodeTemp::Text(value.into())
    }
}

impl From<FlatNode> for FlatNodeTemp {
    fn from(value: FlatNode) -> Self {
        FlatNodeTemp::FlatNode(value)
    }
}

#[derive(Debug)]
pub struct P {
    parts: Vec<ParaParts>
}

#[derive(Debug)]
pub enum ParaParts {
    Text(SCow),
    Link(Link),
    /// Emphasis. May be <em> or <i> 
    Em(SCow),
    /// May be <strong> or <b>
    Strong(SCow)
}

/// Note: Will store empty strings for undefined attributes.
#[derive(Debug)]
pub struct Link {
    pub text: SCow,
    pub href: SCow,
    pub title: SCow,
    pub alt: SCow,
}

#[derive(Debug)]
pub struct Pre {
    pub text: SCow,
}

#[derive(Debug)]
pub struct Heading {
    // HTML headings can be <h1>-<h6>
    level: u8,

    text: SCow,
}