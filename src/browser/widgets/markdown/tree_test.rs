#![cfg(test)]

use indoc::indoc;
use pretty_assertions::assert_eq;

// Weird. The text inside ![] is returned as a Text event, not some metadata in the Image start tag.
// I guess this is to match the pattern for a Link?
#[test]
fn image_end_tag() {
    let md = indoc!{"
        Here is some markdown.

        ![img](https://examplecat.com/cat.png)
        ![](https://examplecat.com/cat.png)

        [![img](https://examplecat.com/cat.png)](https://examplecat.com)
    "};

    assert_eq!( event_debug(&md), &[
        "Start(Paragraph)",
        "Text(Borrowed(\"Here is some markdown.\"))",
        "End(Paragraph)",
        "Start(Paragraph)",
        "Start(Image { link_type: Inline, dest_url: Borrowed(\"https://examplecat.com/cat.png\"), title: Borrowed(\"\"), id: Borrowed(\"\") })",
        "Text(Borrowed(\"img\"))",
        "End(Image)",
        "SoftBreak",
        "Start(Image { link_type: Inline, dest_url: Borrowed(\"https://examplecat.com/cat.png\"), title: Borrowed(\"\"), id: Borrowed(\"\") })",
        "End(Image)",
        "End(Paragraph)",
        "Start(Paragraph)",
        "Start(Link { link_type: Inline, dest_url: Borrowed(\"https://examplecat.com\"), title: Borrowed(\"\"), id: Borrowed(\"\") })",
        "Start(Image { link_type: Inline, dest_url: Borrowed(\"https://examplecat.com/cat.png\"), title: Borrowed(\"\"), id: Borrowed(\"\") })",
        "Text(Borrowed(\"img\"))",
        "End(Image)",
        "End(Link)",
        "End(Paragraph)",
    ]);
}

fn event_debug(md: &str) -> Vec<String> {
    let mut out: Vec<String> = vec![];

    let parser = pulldown_cmark::Parser::new(md);
    for event in parser {
        out.push(format!("{event:?}"));
    }

    out
}