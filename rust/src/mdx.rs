//! MDX support.
//!
//! comrak knows nothing about MDX, and teaching it would mean forking a
//! CommonMark parser. markdown-rs does have an MDX parser but is not built for
//! round-tripping, so it is used for exactly one thing: telling us the byte
//! spans of the MDX constructs. Those spans are swapped for placeholders that
//! comrak carries through untouched, comrak formats the Markdown around them,
//! and the original bytes are pasted back.
//!
//! MDX interiors are frozen. `<Callout>**bold**</Callout>` keeps whatever the
//! author wrote inside it, because the interior of a JSX element is the
//! component's business, and reformatting it can change what the component
//! receives.

use markdown::mdast::Node;
use markdown::{ParseOptions, to_mdast};

use crate::format::{Settings, commonmark};

/// Byte span of an MDX construct. `flow` marks block-level constructs, which
/// stand alone between blank lines; the others sit inside a paragraph.
#[derive(Debug)]
struct Span {
    start: usize,
    end: usize,
    flow: bool,
}

pub fn format(input: &str, settings: &Settings) -> Result<String, String> {
    let tree = to_mdast(input, &ParseOptions::mdx())
        .map_err(|err| format!("could not parse MDX: {err}"))?;

    let mut spans = Vec::new();
    collect(&tree, &mut spans)?;
    spans.sort_by_key(|span| span.start);

    let marks = Marks::for_input(input);
    let (masked, blobs) = mask(input, &spans, &marks);

    let formatted = commonmark(&masked, settings)?;
    let restored = restore(&formatted, &spans, &blobs, &marks);

    // comrak separates two lists that would otherwise merge with an HTML
    // comment. HTML comments are not valid MDX, so translate it to an MDX
    // expression, which is also a comment and still splits the lists when the
    // document is parsed again.
    Ok(restored.replace("<!-- end list -->", "{/* end list */}"))
}

/// Walk the tree recording MDX nodes. Does not recurse into them: the whole
/// construct, children included, is one opaque blob.
fn collect(node: &Node, out: &mut Vec<Span>) -> Result<(), String> {
    let flow = match node {
        Node::MdxjsEsm(_) | Node::MdxJsxFlowElement(_) | Node::MdxFlowExpression(_) => Some(true),
        Node::MdxJsxTextElement(_) | Node::MdxTextExpression(_) => Some(false),
        _ => None,
    };

    if let Some(flow) = flow {
        let position = node
            .position()
            .ok_or_else(|| "MDX node has no source position".to_string())?;
        out.push(Span {
            start: position.start.offset,
            end: position.end.offset,
            flow,
        });
        return Ok(());
    }

    if let Some(children) = node.children() {
        for child in children {
            collect(child, out)?;
        }
    }
    Ok(())
}

/// The two placeholder shapes, chosen so neither occurs in the input.
///
/// Flow spans occupy whole lines, so an HTML comment is the natural stand-in:
/// comrak parses it as an HTML block and reproduces it verbatim at the right
/// indentation.
///
/// Inline spans must not be HTML-comment shaped. A `<!--` at the start of a
/// line opens a CommonMark HTML *block*, which would promote an inline MDX
/// expression to block level and tear the paragraph around it in half. A token
/// delimited by private-use codepoints is ordinary text as far as comrak is
/// concerned: it never escapes it, never breaks a line inside it, and never
/// lets it interfere with emphasis or link parsing.
/// Padding for inline placeholders. An inert ASCII letter, so that one
/// character of padding costs comrak's line-width accounting exactly one byte.
const FILL: char = 'm';

struct Marks {
    nonce: u32,
    open: char,
    close: char,
}

impl Marks {
    fn for_input(input: &str) -> Self {
        let mut nonce = 0;
        while input.contains(&format!("<!--mdfmt{nonce}:")) {
            nonce += 1;
        }

        // Walk the private use area in pairs until both codepoints are free.
        let mut code = 0xE000u32;
        let (open, close) = loop {
            let open = char::from_u32(code).expect("private use area codepoint");
            let close = char::from_u32(code + 1).expect("private use area codepoint");
            if !input.contains(open) && !input.contains(close) {
                break (open, close);
            }
            code += 2;
        };

        Self { nonce, open, close }
    }

    fn flow(&self, index: usize) -> String {
        format!("<!--mdfmt{}:{}-->", self.nonce, index)
    }

    /// An inline placeholder, padded out to the byte length of the blob it
    /// stands in for so comrak wraps the paragraph where it would have wrapped
    /// with the real text in place. comrak measures its columns in bytes, so
    /// that is the unit here too. Blobs shorter than the bare token, and blobs
    /// spanning several lines, get no padding.
    fn inline(&self, index: usize, width: usize) -> String {
        let mut token = format!("{}{}:{}{}", self.open, self.nonce, index, self.close);
        let padding = width.saturating_sub(token.len());
        token.extend(std::iter::repeat_n(FILL, padding));
        token
    }

    fn placeholder(&self, index: usize, span: &Span, blob: &str) -> String {
        if span.flow {
            self.flow(index)
        } else if blob.contains('\n') {
            self.inline(index, 0)
        } else {
            self.inline(index, blob.len())
        }
    }
}

/// An MDX construct held out of comrak's reach, plus the indentation it sat at
/// in the source. The indentation is needed to re-indent the blob when comrak
/// moves the placeholder: the blob's own continuation lines still carry the
/// old indentation, and pasting them under a new one without stripping the old
/// makes the blob creep further right on every run.
struct Blob<'a> {
    text: &'a str,
    indent: &'a str,
}

/// Replace every span with its placeholder, returning the masked document and
/// the blobs that were taken out of it.
fn mask<'a>(input: &'a str, spans: &[Span], marks: &Marks) -> (String, Vec<Blob<'a>>) {
    let mut masked = String::with_capacity(input.len());
    let mut blobs = Vec::with_capacity(spans.len());
    let mut cursor = 0;

    for (index, span) in spans.iter().enumerate() {
        let text = &input[span.start..span.end];
        masked.push_str(&input[cursor..span.start]);
        masked.push_str(&marks.placeholder(index, span, text));
        blobs.push(Blob {
            text,
            indent: source_indent(input, span.start),
        });
        cursor = span.end;
    }
    masked.push_str(&input[cursor..]);

    (masked, blobs)
}

/// The whitespace between the start of `offset`'s line and `offset`, or the
/// empty string when anything else precedes it on that line.
fn source_indent(input: &str, offset: usize) -> &str {
    let line_start = input[..offset].rfind('\n').map_or(0, |at| at + 1);
    let prefix = &input[line_start..offset];
    if prefix.chars().all(|c| matches!(c, ' ' | '\t' | '>')) {
        prefix
    } else {
        ""
    }
}

/// Paste the original spans back over their placeholders.
///
/// Works a line at a time so a multi-line blob can be re-indented to match
/// where comrak put the placeholder. A `<Callout>` nested in a list item comes
/// back indented two spaces on its first line only; without this, its
/// continuation lines land at column zero and fall out of the list.
fn restore(formatted: &str, spans: &[Span], blobs: &[Blob], marks: &Marks) -> String {
    let placeholders: Vec<String> = spans
        .iter()
        .zip(blobs)
        .enumerate()
        .map(|(index, (span, blob))| marks.placeholder(index, span, blob.text))
        .collect();

    let mut out = String::with_capacity(formatted.len());

    for (line_index, line) in formatted.split('\n').enumerate() {
        if line_index > 0 {
            out.push('\n');
        }

        let indent: String = line
            .chars()
            .take_while(|c| matches!(c, ' ' | '\t' | '>'))
            .collect();

        let mut rest = line;
        'line: loop {
            // Expand the placeholder that appears earliest in what is left of
            // the line.
            let next = placeholders
                .iter()
                .enumerate()
                .filter_map(|(index, placeholder)| {
                    rest.find(placeholder.as_str()).map(|at| (at, index))
                })
                .min();

            let Some((at, index)) = next else {
                out.push_str(rest);
                break 'line;
            };

            out.push_str(&rest[..at]);
            out.push_str(&reindent(&blobs[index], &indent));
            rest = &rest[at + placeholders[index].len()..];
        }
    }

    out
}

/// Re-base a blob's continuation lines from the indentation it had in the
/// source onto the indentation comrak gave its placeholder. The first line
/// keeps whatever comrak already emitted before the placeholder.
fn reindent(blob: &Blob, indent: &str) -> String {
    if !blob.text.contains('\n') || blob.indent == indent {
        return blob.text.to_string();
    }

    let mut out = String::with_capacity(blob.text.len());
    for (index, line) in blob.text.split('\n').enumerate() {
        if index > 0 {
            out.push('\n');
            if !line.trim().is_empty() {
                out.push_str(indent);
            }
        }
        out.push_str(if index == 0 {
            line
        } else {
            line.strip_prefix(blob.indent).unwrap_or(line)
        });
    }
    out
}
