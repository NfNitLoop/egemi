#![cfg(test)]

use crate::browser::parsers::html as parse_html;
use indoc::indoc;
use pretty_assertions::{assert_eq};

/// Test that:
///  * </p> is respected.
///  * Text outside of paragraphs is kept.
///  * <title> should be removed. (default is to include it, oddly)
///  * <script> is removed.
///  * <!-- comments --> are removed.
/// Noteworthy: 
///  * Some output paragraphs include a leading space, but that is apparently not significant in CommonMark and will be removed.
#[test]
fn simple_example() {
   let example = indoc! { r#"
        <html>
        <head>
            <title>Title should be removed</title>
            <style>foo { color: red; }</style>
            <script>
                alert("Hello!");
            </script>
        </head>
        <body>
        <script>
            alert("In body");
        </script>
        <!--
            Here is a comment
        -->
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
    "# };

    let out = parse_html::to_md(example);
    assert_eq!(out, indoc! { r#"
        A bit of this & that
        ==========

         Some text and some more text

        This is a *paragraph*

        This is an H1
        ==========

         And yet more text.

         Here is a paragraph with an inline [link](https://www.google.com) and ![](https://examplecat.com/cat.png) image.

        [A link outside of a paragraph](https://examplecat.com/)

        an h2
        ----------"#
    });
}