//! Aligning GFM tables: every one in the document when it is formatted, and
//! the one under the cursor while it is being edited.
//!
//! comrak renders a table with single spaces around its cells, which is valid
//! Markdown and unreadable in a plain-text editor, so the padding is put back
//! here. Both entry points share the work; the editing one additionally says
//! where the cursor went, since Neovim owns the two things only it can know:
//! which buffer line the cursor is on, and how to write the result back
//! without disturbing marks and undo history.
//!
//! None of what happens here is a Markdown parser:
//! comrak parses the buffer, and this module reads the answer off comrak's
//! AST. Which lines are a table, where its cells begin and end, and how its
//! columns are aligned are all questions comrak has already answered by the
//! time we look, and `sourcepos` says exactly where in the buffer each answer
//! came from.
//!
//! The consequence of deferring is worth stating plainly: a run of lines is a
//! table when comrak says it is, and not before. A header row whose delimiter
//! row has a different number of cells is not a table in GFM, so it is left
//! alone rather than guessed at. The same goes for a row carrying more cells
//! than the header declares: GFM discards the surplus, so rewriting the row
//! would delete text the author can still see, and the table is left alone
//! until the header catches up.
//!
//! Positions follow Neovim's own conventions at the boundary: rows are
//! 1-based, columns are 0-based byte offsets. comrak's own `sourcepos` columns
//! are 1-based byte offsets, so they are converted on the way in and out.

use comrak::nodes::{AstNode, NodeValue, Sourcepos, TableAlignment};
use comrak::{Arena, parse_document};
use unicode_width::UnicodeWidthStr;

use crate::format::{Settings, options};

/// Narrowest a column may be. Anything less leaves no room for a `---`
/// delimiter, which every column needs to keep the table a table.
const MIN_WIDTH: usize = 3;

/// The result of formatting the table under the cursor.
pub struct TableEdit {
    /// 1-based, inclusive range of buffer lines to replace.
    pub first: usize,
    pub last: usize,
    /// Replacement text for `first..=last`, in order.
    pub lines: Vec<String>,
    /// Where the cursor should end up. The row never moves: table formatting
    /// never inserts or removes rows.
    pub cursor_row: usize,
    /// 0-based byte column.
    pub cursor_col: usize,
}

/// One cell, as comrak found it in the buffer.
struct Cell<'a> {
    /// The cell's text with its surrounding padding removed, taken verbatim
    /// from the buffer. Comrak's own CommonMark output would do here instead,
    /// but it normalizes as it goes: a half-typed `**bol` comes back as
    /// `\*\*bol`, which is not something to do to someone mid-keystroke.
    text: &'a str,
    /// 0-based byte offset, within the line, of `text`.
    start: usize,
}

/// One row of the table, and the buffer line it sits on.
struct Row<'a> {
    line: usize,
    /// Whatever the buffer puts in front of the row: the `> ` of a
    /// blockquote, or the indent of a list item.
    prefix: &'a str,
    cells: Vec<Cell<'a>>,
}

/// A table comrak found, read out of the document it was found in.
struct Table<'a> {
    /// 1-based, inclusive line range the table occupies.
    first: usize,
    last: usize,
    /// The line the delimiter row is on. It is not a node of its own: comrak
    /// folds it into the table's alignments, and it is always the line after
    /// the header.
    separator: usize,
    /// Taken from the header and used for every line, so a row that was quoted
    /// or indented more loosely than its header comes back matching it.
    prefix: &'a str,
    alignments: Vec<TableAlignment>,
    widths: Vec<usize>,
    rows: Vec<Row<'a>>,
}

/// Align every table in a document.
///
/// This is the pass that makes `md-fmt`'s own output readable: comrak renders
/// `| a | b |` with whatever spacing the cells happen to need, and this puts
/// the columns back under one another.
pub fn align(input: &str, settings: &Settings) -> String {
    let lines = buffer_lines(input);
    let arena = Arena::new();
    let root = parse_document(&arena, input, &options(settings));

    let mut out: Vec<String> = lines.iter().map(|line| line.to_string()).collect();
    for node in root.descendants() {
        // Tables cannot nest, so no two of these overlap.
        let Some(table) = Table::read(node, &lines) else {
            continue;
        };
        for (index, line) in table.render(&lines).into_iter().enumerate() {
            out[table.first - 1 + index] = line;
        }
    }

    let mut text = out.join("\n");
    if input.ends_with('\n') {
        text.push('\n');
    }
    text
}

/// Align the table under `(row, col)`, if there is one, leaving the rest of
/// the document alone.
///
/// `input` is the whole buffer, lines joined by `\n` with a trailing one: the
/// same convention the rest of the crate uses for stdin. `row` is 1-based and
/// `col` a 0-based byte offset, matching `nvim_win_get_cursor`.
pub fn format(input: &str, settings: &Settings, row: usize, col: usize) -> Option<TableEdit> {
    let lines = buffer_lines(input);
    if row == 0 || row > lines.len() {
        return None;
    }

    let arena = Arena::new();
    let root = parse_document(&arena, input, &options(settings));
    let table = root
        .descendants()
        .filter_map(|node| Table::read(node, &lines))
        .find(|table| table.first <= row && row <= table.last)?;

    let replacement = table.render(&lines);
    let cursor_col = table
        .cursor(row, col)
        .unwrap_or_else(|| col.min(replacement[row - table.first].len()));

    Some(TableEdit {
        first: table.first,
        last: table.last,
        lines: replacement,
        cursor_row: row,
        cursor_col,
    })
}

impl<'a> Table<'a> {
    /// Read `node`, if it is a table this module can rewrite.
    fn read<'b>(node: &'b AstNode<'b>, lines: &[&'a str]) -> Option<Self> {
        let NodeValue::Table(table) = &node.data().value else {
            return None;
        };
        let alignments = table.alignments.clone();
        let span = node.data().sourcepos;

        let rows = collect_rows(node, lines)?;
        let separator = rows.first()?.line + 1;
        if separator > span.end.line {
            return None;
        }

        Some(Self {
            first: span.start.line,
            last: span.end.line,
            separator,
            prefix: rows.first()?.prefix,
            widths: widths(&rows, &alignments),
            alignments,
            rows,
        })
    }

    /// The table's lines, padded out to its column widths.
    fn render(&self, lines: &[&str]) -> Vec<String> {
        (self.first..=self.last)
            .map(|line| {
                if line == self.separator {
                    render(self.prefix, &delimiters(&self.alignments, &self.widths))
                } else if let Some(row) = self.rows.iter().find(|row| row.line == line) {
                    render(self.prefix, &padded(row, &self.widths))
                } else {
                    // Unreachable while comrak gives every table line a row,
                    // and harmless if that ever changes: the line is kept.
                    lines[line - 1].to_string()
                }
            })
            .collect()
    }

    /// Where the cursor lands once the table has been rewritten: the same
    /// offset into the same cell it was in before, which is what keeps typing
    /// in a cell from being interrupted by the realignment around it.
    ///
    /// `None` when the cursor is not on a row, which means it is on the
    /// delimiter line.
    fn cursor(&self, row: usize, col: usize) -> Option<usize> {
        let cells = &self
            .rows
            .iter()
            .find(|candidate| candidate.line == row)?
            .cells;

        let mut column = 0;
        let mut offset = 0;
        for (index, cell) in cells.iter().enumerate() {
            if col >= cell.start {
                column = index;
                offset = (col - cell.start).min(cell.text.len());
            }
        }

        // Two for the `| ` the line opens with, three for each ` | ` crossed
        // on the way to this column.
        let start = self.prefix.len()
            + 2
            + self.widths[..column.min(self.widths.len())]
                .iter()
                .map(|width| width + 3)
                .sum::<usize>();
        Some(start + offset)
    }
}

/// The buffer's lines. The input carries the trailing newline every document
/// ends with, which is not a line of its own.
fn buffer_lines(input: &str) -> Vec<&str> {
    let mut lines: Vec<&str> = input.split('\n').collect();
    if lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }
    lines
}

/// Every row of the table, with its cells read out of the buffer.
///
/// `None` when any row carries text beyond its last cell. GFM ignores the
/// surplus, so it has no cell to be rendered from, and rewriting the row would
/// silently delete it.
fn collect_rows<'a, 'b>(table: &'b AstNode<'b>, lines: &[&'a str]) -> Option<Vec<Row<'a>>> {
    let mut rows = Vec::new();

    for node in table.children() {
        let data = node.data();
        if !matches!(data.value, NodeValue::TableRow(_)) {
            continue;
        }

        let index = data.sourcepos.start.line;
        let line = lines.get(index - 1)?;
        let mut cells = Vec::new();
        let mut end = 0;

        for child in node.children() {
            let span = child.data().sourcepos;
            if !matches!(child.data().value, NodeValue::TableCell) {
                continue;
            }
            cells.push(cell(line, span));
            end = end.max(span.end.column.min(line.len()));
        }

        if !trailing(line, end) {
            return None;
        }
        rows.push(Row {
            line: index,
            prefix: prefix(line, data.sourcepos.start.column),
            cells,
        });
    }

    (!rows.is_empty()).then_some(rows)
}

/// The cell comrak located at `span`, trimmed of the padding around it.
///
/// A row shorter than the table is completed by comrak with cells that have no
/// text behind them, whose span is the single byte of the pipe that ended the
/// row. There is nothing else a lone unescaped pipe can be, so it stands in
/// for the empty cell it is.
fn cell(line: &str, span: Sourcepos) -> Cell<'_> {
    let start = span.start.column.saturating_sub(1).min(line.len());
    let end = span.end.column.min(line.len());
    // An empty cell still reports where it is, so that a cursor to the left of
    // it is not mistaken for a cursor inside it.
    let empty = Cell { text: "", start };

    if start > end || !line.is_char_boundary(start) || !line.is_char_boundary(end) {
        return empty;
    }
    let raw = &line[start..end];
    let text = raw.trim();
    if text.is_empty() || text == "|" {
        return empty;
    }
    Cell {
        text,
        start: start + (raw.len() - raw.trim_start().len()),
    }
}

/// Whether everything after a row's last cell is the pipe that closed it.
fn trailing(line: &str, end: usize) -> bool {
    let rest = line[end.min(line.len())..].trim_start();
    rest.strip_prefix('|').unwrap_or(rest).trim().is_empty()
}

/// The line up to where the row's own content starts, which is where comrak
/// says the containing blockquote or list item has finished with it.
fn prefix(line: &str, column: usize) -> &str {
    let end = column.saturating_sub(1).min(line.len());
    if line.is_char_boundary(end) {
        &line[..end]
    } else {
        ""
    }
}

/// The display width of each column: the widest cell in it, and never so
/// narrow that the delimiter row cannot be written.
fn widths(rows: &[Row], alignments: &[TableAlignment]) -> Vec<usize> {
    let mut widths = vec![MIN_WIDTH; alignments.len()];
    for row in rows {
        for (column, cell) in row.cells.iter().enumerate().take(widths.len()) {
            widths[column] = widths[column].max(cell.text.width());
        }
    }
    widths
}

fn delimiters(alignments: &[TableAlignment], widths: &[usize]) -> Vec<String> {
    alignments
        .iter()
        .zip(widths)
        .map(|(alignment, width)| match alignment {
            TableAlignment::Center => format!(":{}:", dashes(width.saturating_sub(2))),
            TableAlignment::Right => format!("{}:", dashes(width.saturating_sub(1))),
            TableAlignment::Left => format!(":{}", dashes(width.saturating_sub(1))),
            TableAlignment::None => dashes(*width),
        })
        .collect()
}

fn dashes(count: usize) -> String {
    "-".repeat(count.max(1))
}

/// The row's cells, each padded out to its column's width.
fn padded(row: &Row, widths: &[usize]) -> Vec<String> {
    widths
        .iter()
        .enumerate()
        .map(|(column, width)| {
            let text = row.cells.get(column).map_or("", |cell| cell.text);
            format!("{text}{}", " ".repeat(width.saturating_sub(text.width())))
        })
        .collect()
}

fn render(prefix: &str, cells: &[String]) -> String {
    format!("{prefix}| {} |", cells.join(" | "))
}
