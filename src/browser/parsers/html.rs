//! Parsing HTML is a pain.
//! And I want to render Markdown too.
//! So let's just convert to markdown and then show that.

use std::collections::HashMap;

use html2md::{Handle, StructuredPrinter, TagHandler, TagHandlerFactory};
use log::debug;

mod html_test;


pub fn to_md(html: &str) -> String {
    let mut tag_map: HashMap<String, Box<dyn TagHandlerFactory>> = HashMap::new();

    let skip_tag = Box::new(SkipTagFactory);
    tag_map.insert("head".into(), skip_tag.clone());
    tag_map.insert("script".into(), skip_tag);

    let out = html2md::parse_html_custom(html, &tag_map);

    out
}

/// By default, html2md will parse & show <head> and <title> tags, but we usually just want to show the document.
struct SkipTag;

impl TagHandler for SkipTag {
    fn handle(&mut self, tag: &Handle, _printer: &mut StructuredPrinter) { 
        eprintln!("Skipping tag: {:#?}", tag.data);
        debug!("Skipping tag: {:#?}", tag.data);
    }

    fn after_handle(&mut self, _printer: &mut StructuredPrinter) { }

    fn skip_descendants(&self) -> bool { true }
}

#[derive(Clone, Debug)]
struct SkipTagFactory;

impl TagHandlerFactory for SkipTagFactory {
    fn instantiate(&self) -> Box<dyn html2md::TagHandler> {
        Box::new(SkipTag)
    }
}