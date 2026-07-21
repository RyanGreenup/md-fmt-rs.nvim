//! Realigning the table under the cursor.
//!
//! The Neovim side sends the whole buffer and a cursor position and expects
//! back a line range, its replacement, and where the cursor went. These tests
//! exercise that contract directly, so the Lua tests only have to prove the
//! wiring.

use md_fmt::format::Settings;
use md_fmt::table::{TableEdit, format};

/// Format a buffer given as lines, with the cursor at `(row, col)`.
fn run(lines: &[&str], row: usize, col: usize) -> Option<TableEdit> {
    let input = format!("{}\n", lines.join("\n"));
    format(&input, &Settings::default(), row, col)
}

/// The replacement lines, which is all most of these care about.
fn realign(lines: &[&str], row: usize, col: usize) -> Vec<String> {
    run(lines, row, col)
        .expect("a table under the cursor")
        .lines
}

#[test]
fn pads_cells_to_the_widest_in_their_column() {
    let out = realign(&["| Name|Age |", "|:--|--:|", "|Ryan|42|"], 3, 6);

    assert_eq!(out, ["| Name | Age |", "| :--- | --: |", "| Ryan | 42  |"]);
}

#[test]
fn realigning_is_idempotent() {
    let once = realign(&["| Name|Age |", "|:--|--:|", "|Ryan|42|"], 3, 6);
    let borrowed: Vec<&str> = once.iter().map(String::as_str).collect();

    assert_eq!(realign(&borrowed, 3, 6), once);
}

#[test]
fn only_the_table_under_the_cursor_is_touched() {
    let edit = run(
        &["before", "", "| a|b |", "|-|-|", "|1|2|", "", "after"],
        5,
        3,
    )
    .expect("a table under the cursor");

    assert_eq!((edit.first, edit.last), (3, 5));
    assert_eq!(edit.lines.len(), 3);
}

#[test]
fn a_row_shorter_than_the_header_is_padded_out() {
    let out = realign(&["| a | b | c |", "|-|-|-|", "| 1 |"], 3, 3);

    assert_eq!(
        out,
        [
            "| a   | b   | c   |",
            "| --- | --- | --- |",
            "| 1   |     |     |"
        ]
    );
}

#[test]
fn display_width_decides_the_padding() {
    // The crab is two columns wide and four bytes long. Padding to bytes
    // would leave the column ragged in the buffer.
    let out = realign(&["| key | value |", "|---|---|", "| emoji | 🦀 |"], 3, 3);

    assert_eq!(
        out,
        [
            "| key   | value |",
            "| ----- | ----- |",
            "| emoji | 🦀    |"
        ]
    );
}

#[test]
fn pipes_that_are_not_separators_stay_in_their_cell() {
    // An escaped pipe and a pipe inside inline code are both cell content,
    // and comrak is the one that knows the difference.
    let out = realign(
        &[
            "| key | value |",
            "|---|---|",
            "| pipe | a\\|b |",
            "| code | `a\\|b` |",
        ],
        3,
        3,
    );

    assert_eq!(
        out,
        [
            "| key  | value  |",
            "| ---- | ------ |",
            "| pipe | a\\|b   |",
            "| code | `a\\|b` |",
        ]
    );
}

#[test]
fn alignment_markers_survive() {
    let out = realign(
        &["| a | b | c | d |", "|:-|-:|:-:|-|", "| 1 | 2 | 3 | 4 |"],
        1,
        2,
    );

    assert_eq!(out[1], "| :-- | --: | :-: | --- |");
}

#[test]
fn a_blockquoted_table_keeps_its_quote() {
    let out = realign(&["> | a|b |", "> |-|-|", "> |1|2|"], 2, 4);

    assert_eq!(
        out,
        ["> | a   | b   |", "> | --- | --- |", "> | 1   | 2   |"]
    );
}

#[test]
fn a_table_indented_under_a_list_item_keeps_its_indent() {
    let out = realign(&["- item", "", "  | a|b |", "  |-|-|", "  |1|2|"], 5, 5);

    assert_eq!(
        out,
        ["  | a   | b   |", "  | --- | --- |", "  | 1   | 2   |"]
    );
}

#[test]
fn the_cursor_follows_its_cell() {
    let edit = run(&["| Name|Age |", "|:--|--:|", "|Ryan|42|"], 3, 6).expect("a table");

    // The cursor sat on the `4` of `42`; it should still be on it.
    assert_eq!(edit.cursor_row, 3);
    let line = &edit.lines[edit.cursor_row - edit.first];
    assert_eq!(&line[edit.cursor_col..edit.cursor_col + 2], "42");
}

#[test]
fn the_cursor_keeps_its_offset_inside_a_widened_cell() {
    // Between the `a` and the `b` of the first cell.
    let edit = run(&["|ab|c|", "|-|-|", "| wide | c |"], 1, 2).expect("a table");

    let line = &edit.lines[0];
    assert_eq!(&line[..edit.cursor_col], "| a");
}

#[test]
fn the_cursor_stays_put_in_a_row_that_had_cells_added_to_it() {
    // The two cells comrak completed this row with are not where the cursor
    // is, however far to the right of them the padding puts it.
    let edit = run(&["| a | b | c |", "|-|-|-|", "| 1 |"], 3, 2).expect("a table");

    let line = &edit.lines[2];
    assert_eq!(&line[..edit.cursor_col + 1], "| 1");
}

#[test]
fn a_table_written_without_outer_pipes_is_still_a_table() {
    // And a following line with no pipe at all is a row of it, with one cell,
    // which is what GFM says it is.
    let out = realign(&["A | B", "--- | ---", "one | two", "four"], 3, 6);

    assert_eq!(
        out,
        [
            "| A    | B   |",
            "| ---- | --- |",
            "| one  | two |",
            "| four |     |",
        ]
    );
}

#[test]
fn what_is_not_a_table_is_left_alone() {
    // A delimiter row has to match the header's cell count to make a table,
    // and a fenced code block is not markup at all. Neither is something to
    // guess at: comrak's answer is the answer.
    assert!(run(&["| header | only |", "| body | no delimiter |"], 1, 3).is_none());
    assert!(run(&["```", "| a | b |", "|---|---|", "| 1 | 2 |", "```"], 3, 3).is_none());
    assert!(run(&["just prose", "", "and more"], 1, 3).is_none());
}

#[test]
fn a_row_with_more_cells_than_the_header_is_left_alone() {
    // GFM ignores the surplus cell, so there is no rendering of this table
    // that keeps `three` in it. Leaving the lines untouched until the header
    // grows a column beats deleting what the author just typed.
    assert!(run(&["| a | b |", "|-|-|", "| 1 | 2 | three |"], 3, 3).is_none());
}

#[test]
fn a_cursor_outside_the_buffer_is_not_a_table() {
    assert!(run(&["| a | b |", "|-|-|"], 9, 0).is_none());
    assert!(run(&["| a | b |", "|-|-|"], 0, 0).is_none());
}
