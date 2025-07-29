#![cfg(test)]

// :( `tl` crate treats *everything* after a <p> as a paragraph unless it sees a </p> boo.

use crate::browser::html::FlatParser;

#[test]
fn as_documented() {
    let dom = tl::parse(r#"<p id="text">Hello</p>"#, tl::ParserOptions::default()).unwrap();
    let parser = dom.parser();
    let element = dom.get_element_by_id("text")
    .expect("Failed to find element")
    .get(parser)
    .unwrap();

    assert_eq!(element.inner_text(parser), "Hello");

}

// The parser keeps whitespace when fetching .inner_text()
#[test]
fn keeps_whitespace() {
    let raw = r#"
    <blah><div>
        <p>This is a <i>paragraph</i></p>
        <p> This is 
            <b>another</b> paragraph.
    </div></blah>"#;

    let dom = tl::parse(&raw, tl::ParserOptions::default()).unwrap();
    let paragraphs: Vec<_> = dom.query_selector("p")
        .map(|it| it.collect())
        .unwrap_or_default();
    let paragraphs: Vec<_> = paragraphs.into_iter()
        .flat_map(|p| p.get(dom.parser()))
        .map(|it| it.inner_text(dom.parser()))
        .collect();
    ;

    // println!("{paragraphs:#?}");
    assert_eq!(paragraphs, &[
        "This is a paragraph", 
        // Hmm.... The parser keeps all whitespace
        " This is \n            another paragraph.\n    "
    ]);
}

#[test]
fn simple_parse() {
    let example = r#"
        <html>
        <head>
        <title>"foo"</title>
        </head>
        <body>
        <div>
            <h1>A bit of this &amp; that</h1>
            Some text
            and some more text
            <p>This is <span>a</span> <i>paragraph</i>
            <h1>This is an H1</h1> 
            And yet more text.
            
            <p>
                Here is a paragraph with an inline
                <a href="https://www.google.com">link</a>
                and
                <img src="https://examplecat.com/cat.png"/>
                image.
            </p>
            <a href="https://examplecat.com/">A link outside of a paragraph</a>
            <h2>an h2</h2>
        </div>
        </body>
        </html>
    "#;

    let dom = tl::parse(&example, tl::ParserOptions::default()).unwrap();

    let parser = FlatParser;
    let parts = parser.parse(&dom);
    println!("{parts:#?}");
}
