//! Behaviour the Neovim side depends on.
//!
//! The load-bearing property is idempotency: an editor formatter that keeps
//! changing the buffer every time it runs is worse than no formatter, because
//! it turns every save into a diff.

use md_fmt::format::{Settings, format};

const KITCHEN_SINK: &str = include_str!("fixtures/kitchen-sink.mdx");
const PLAIN: &str = include_str!("fixtures/plain.md");

fn mdx() -> Settings {
    Settings {
        mdx: true,
        ..Settings::default()
    }
}

fn run(input: &str, settings: &Settings) -> String {
    format(input, settings).expect("fixture formats cleanly")
}

#[test]
fn formatting_is_idempotent() {
    for (input, settings) in [(KITCHEN_SINK, mdx()), (PLAIN, Settings::default())] {
        let once = run(input, &settings);
        let twice = run(&once, &settings);
        assert_eq!(once, twice, "second pass changed the document");
    }
}

#[test]
fn mdx_blobs_survive_verbatim() {
    let out = run(KITCHEN_SINK, &mdx());

    for blob in [
        "import Widget from './widget.js'",
        "{props.count}",
        "<Chip label=\"x\" />",
        "export const answer = 42",
        "<Callout kind=\"info\">\nSome **markdown**    frozen inside the blob.\n</Callout>",
    ] {
        assert!(out.contains(blob), "lost MDX blob: {blob:?}\n---\n{out}");
    }
}

#[test]
fn mdx_blob_in_a_list_item_keeps_its_indentation() {
    let input = "- item:\n\n  <Callout>\n  first\n  second\n  </Callout>\n";
    let out = run(input, &mdx());

    assert!(
        out.contains("  <Callout>\n  first\n  second\n  </Callout>"),
        "blob fell out of the list item:\n{out}"
    );
}

#[test]
fn setext_headings_become_atx() {
    assert!(run(PLAIN, &Settings::default()).contains("# Heading\n"));
}

#[test]
fn frontmatter_survives() {
    for (input, settings) in [(KITCHEN_SINK, mdx()), (PLAIN, Settings::default())] {
        let out = run(input, &settings);
        assert!(
            out.starts_with("---\ntitle: "),
            "mangled frontmatter:\n{out}"
        );
    }
}

#[test]
fn frontmatter_can_be_turned_off() {
    let settings = Settings {
        frontmatter: None,
        ..Settings::default()
    };
    let out = run("---\ntitle: x\n---\n\nbody\n", &settings);
    assert!(
        !out.starts_with("---\ntitle"),
        "expected a thematic break:\n{out}"
    );
}

#[test]
fn prose_wraps_at_the_requested_width() {
    let settings = Settings {
        width: 40,
        ..Settings::default()
    };
    let out = run(PLAIN, &settings);
    let longest = out
        .lines()
        // Tables and links are not breakable, so they are allowed to overflow.
        .filter(|line| !line.contains('|') && !line.contains(']'))
        .map(str::len)
        .max()
        .unwrap();
    assert!(
        longest <= 40,
        "line of {longest} bytes exceeded the width:\n{out}"
    );
}

#[test]
fn zero_width_leaves_line_breaks_alone() {
    let settings = Settings {
        width: 0,
        ..Settings::default()
    };
    let input = "one two three four five six seven eight nine ten eleven twelve thirteen fourteen fifteen\n";
    assert_eq!(run(input, &settings), input);
}

#[test]
fn html_comments_survive_in_plain_markdown() {
    // The `<!-- end list -->` rewrite is MDX-only: in Markdown the comment is
    // valid and rewriting it to an MDX expression would put literal braces in
    // the document.
    let out = run(PLAIN, &Settings::default());
    assert!(out.contains("<!-- an ordinary HTML comment -->"));
}

#[test]
fn end_list_separators_become_mdx_comments() {
    // Two lists that normalize to the same bullet would merge into one, so
    // comrak parts them with an HTML comment. HTML comments are not valid MDX.
    let out = run("- bullet\n\n* other bullet\n", &mdx());
    assert!(!out.contains("<!--"), "left an HTML comment in MDX:\n{out}");
    assert!(
        out.contains("{/* end list */}"),
        "lost the separator:\n{out}"
    );
}

#[test]
fn blank_lines_are_emptied_but_code_blocks_are_not() {
    let out = run(PLAIN, &Settings::default());

    assert!(
        !out.lines()
            .any(|line| !line.is_empty() && line.trim().is_empty()),
        "left a whitespace-only line:\n{out:?}"
    );
    // The blank line inside the Python block is part of the code.
    assert!(
        out.contains("def f():\n\n    return 1"),
        "mangled code:\n{out}"
    );
}

#[test]
fn trailing_newline_is_preserved() {
    assert_eq!(run("# hi", &Settings::default()), "# hi");
    assert_eq!(run("# hi\n", &Settings::default()), "# hi\n");
}

#[test]
fn empty_input_stays_empty() {
    assert_eq!(run("", &Settings::default()), "");
    assert_eq!(run("", &mdx()), "");
}

#[test]
fn placeholders_do_not_collide_with_the_document() {
    // A document that already contains what the masker would otherwise have
    // used. The HTML comment has to hide in a code block, since MDX rejects
    // HTML comments everywhere else.
    let input = "<Chip />\n\nliteral \u{e000}0:0\u{e001} text\n\n```\n<!--mdfmt0:0-->\n```\n";
    let out = run(input, &mdx());

    assert!(
        out.contains("<!--mdfmt0:0-->"),
        "ate a literal marker:\n{out}"
    );
    assert!(
        out.contains("\u{e000}0:0\u{e001}"),
        "ate a literal marker:\n{out}"
    );
    assert!(out.contains("<Chip />"), "lost the real MDX:\n{out}");
}

#[test]
fn malformed_mdx_is_an_error() {
    let err = format("<Callout>\nunclosed\n", &mdx()).unwrap_err();
    assert!(
        err.contains("could not parse MDX"),
        "unhelpful error: {err}"
    );
}

#[test]
fn angle_brackets_are_fine_without_mdx() {
    // `--mdx` off must not run the MDX parser, so a `<` that is not JSX is not
    // an error. comrak backslash-escapes it, which renders identically.
    let input = "a < b and c > d\n";
    assert_eq!(run(input, &Settings::default()), "a \\< b and c \\> d\n");
}
