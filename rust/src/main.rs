//! md-fmt: the formatting engine behind md-fmt-rs.nvim.
//!
//! Reads a Markdown or MDX document on stdin and writes the formatted document
//! on stdout. On failure it writes a message to stderr and produces nothing on
//! stdout, so the editor can tell the difference between "here is your
//! formatted buffer" and "leave the buffer alone".

use md_fmt::{cli, format, table};

use std::io::{Read, Write};
use std::process::ExitCode;

/// Bad input document.
const EXIT_FORMAT: u8 = 1;
/// Bad command line.
const EXIT_USAGE: u8 = 2;
/// `--table` found no table under the cursor.
const EXIT_NO_TABLE: u8 = 1;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let outcome = match cli::parse(&args) {
        Ok(outcome) => outcome,
        Err(message) => return fail(&message, EXIT_USAGE),
    };

    if let cli::Outcome::Message(message) = outcome {
        print!("{message}");
        return ExitCode::SUCCESS;
    }

    let mut input = String::new();
    if let Err(err) = std::io::stdin().read_to_string(&mut input) {
        return fail(&format!("could not read stdin: {err}"), EXIT_FORMAT);
    }

    match outcome {
        cli::Outcome::Run(settings) => match format::format(&input, &settings) {
            Ok(formatted) => write_stdout(&formatted),
            Err(message) => fail(&message, EXIT_FORMAT),
        },
        cli::Outcome::Table { settings, row, col } => {
            match table::format(&input, &settings, row, col) {
                Some(edit) => write_table_edit(&edit),
                None => ExitCode::from(EXIT_NO_TABLE),
            }
        }
        cli::Outcome::Message(_) => ExitCode::SUCCESS,
    }
}

fn write_stdout(text: &str) -> ExitCode {
    match std::io::stdout().write_all(text.as_bytes()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => fail(&format!("could not write stdout: {err}"), EXIT_FORMAT),
    }
}

/// `first last cursor_row cursor_col`, one replacement line per row after.
fn write_table_edit(edit: &table::TableEdit) -> ExitCode {
    let mut out = format!(
        "{} {} {} {}\n",
        edit.first, edit.last, edit.cursor_row, edit.cursor_col
    );
    for line in &edit.lines {
        out.push_str(line);
        out.push('\n');
    }
    write_stdout(&out)
}

fn fail(message: &str, code: u8) -> ExitCode {
    eprintln!("md-fmt: {message}");
    ExitCode::from(code)
}
