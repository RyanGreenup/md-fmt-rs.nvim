//! The CommonMark half of the formatter: comrak options, and the entry point
//! that dispatches to the MDX pipeline when asked.

use comrak::{Arena, Options, format_commonmark, parse_document};

#[derive(Debug, Clone)]
pub struct Settings {
    /// Parse MDX constructs and hold them out of comrak's reach.
    pub mdx: bool,
    /// Column to wrap prose at. 0 leaves line breaks alone.
    pub width: usize,
    /// Frontmatter fence, or None to treat a leading `---` as ordinary
    /// Markdown.
    pub frontmatter: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            mdx: false,
            width: 80,
            frontmatter: Some("---".to_string()),
        }
    }
}

/// Format a whole document.
///
/// The trailing newline of the input is reproduced: comrak always terminates
/// its output with one, which would otherwise turn a buffer with `noeol` set
/// into a buffer with a trailing blank.
pub fn format(input: &str, settings: &Settings) -> Result<String, String> {
    let formatted = if settings.mdx {
        crate::mdx::format(input, settings)?
    } else {
        commonmark(input, settings)?
    };

    if input.ends_with('\n') || formatted.is_empty() {
        Ok(formatted)
    } else {
        Ok(formatted.trim_end_matches('\n').to_string())
    }
}

/// Run comrak over a document that contains no MDX, or whose MDX has already
/// been masked out.
pub fn commonmark(input: &str, settings: &Settings) -> Result<String, String> {
    let options = options(settings);
    let arena = Arena::new();
    let root = parse_document(&arena, input, &options);

    let mut out = String::with_capacity(input.len());
    format_commonmark(root, &options, &mut out)
        .map_err(|err| format!("comrak failed to render: {err}"))?;

    // comrak renders a table with one space around each cell, which is valid
    // and unreadable. Padding the columns is a second pass over comrak's own
    // output rather than part of the render, since comrak does not offer the
    // hook, and it is the same pass the editor runs while a table is typed.
    //
    // On the MDX path this runs while the blobs are still masked, which is
    // what keeps a `{a || b}` in a cell from being read as two more cells.
    // The cost is that such a cell is padded by the width of its placeholder
    // rather than the width of the blob, so its column comes out a few spaces
    // wide or narrow. Mangling the expression to fix the spacing would be a
    // poor trade.
    Ok(crate::table::align(&blank_lines(&out), settings))
}

/// Empty the whitespace-only lines comrak leaves behind when it indents a
/// loose list item.
///
/// A line of nothing but spaces is never a hard line break (that needs two
/// spaces after actual text), so emptying it cannot change how the document
/// parses. Inside a fenced code block it could change the *contents*, though,
/// so those are skipped.
fn blank_lines(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut fence: Option<(char, usize)> = None;

    for (index, line) in text.split('\n').enumerate() {
        if index > 0 {
            out.push('\n');
        }

        match fence {
            Some(open) if closes_fence(line, open) => fence = None,
            Some(_) => {
                out.push_str(line);
                continue;
            }
            None => fence = opens_fence(line),
        }

        if line.trim().is_empty() {
            continue;
        }
        out.push_str(line);
    }

    out
}

/// The fence character and length if this line opens a fenced code block.
fn opens_fence(line: &str) -> Option<(char, usize)> {
    let rest = line.trim_start();
    let marker = rest.chars().next().filter(|c| matches!(c, '`' | '~'))?;
    let length = rest.chars().take_while(|c| *c == marker).count();
    // An info string may not contain a backtick, which is what keeps inline
    // code out of this.
    if length >= 3 && !(marker == '`' && rest[length..].contains('`')) {
        Some((marker, length))
    } else {
        None
    }
}

fn closes_fence(line: &str, (marker, length): (char, usize)) -> bool {
    let rest = line.trim_start();
    rest.chars().take_while(|c| *c == marker).count() >= length
        && rest.trim_end().chars().all(|c| c == marker)
}

/// The parser and renderer configuration a document is read with. Table
/// realignment reads the buffer with exactly these, so that both halves of the
/// crate agree on what is and is not a table.
pub(crate) fn options(settings: &Settings) -> Options<'static> {
    let mut options = Options::default();

    options.extension.table = true;
    options.extension.strikethrough = true;
    options.extension.tasklist = true;
    options.extension.alerts = true;
    // GFM autolinks render bare URLs back out as `<url>`, which mdxjs-rs
    // parses as the start of a JSX tag rather than an autolink. Emitting
    // that form would make our own output fail to re-parse as MDX.
    options.extension.autolink = !settings.mdx;
    options.extension.footnotes = true;
    options.extension.math_dollars = true;
    options.extension.front_matter_delimiter = settings.frontmatter.clone();

    options.render.width = settings.width;
    // Fenced code blocks survive a round trip; indented ones lose their
    // language and collide with list indentation.
    options.render.prefer_fenced = true;

    options
}
