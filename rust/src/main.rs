//! md-fmt: the formatting engine behind mdfmt.nvim.
//!
//! Reads a Markdown or MDX document on stdin and writes the formatted document
//! on stdout. On failure it writes a message to stderr and produces nothing on
//! stdout, so the editor can tell the difference between "here is your
//! formatted buffer" and "leave the buffer alone".

use md_fmt::{cli, format};

use std::io::{Read, Write};
use std::process::ExitCode;

/// Bad input document.
const EXIT_FORMAT: u8 = 1;
/// Bad command line.
const EXIT_USAGE: u8 = 2;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let settings = match cli::parse(&args) {
        Ok(cli::Outcome::Run(settings)) => settings,
        Ok(cli::Outcome::Message(message)) => {
            print!("{message}");
            return ExitCode::SUCCESS;
        }
        Err(message) => return fail(&message, EXIT_USAGE),
    };

    let mut input = String::new();
    if let Err(err) = std::io::stdin().read_to_string(&mut input) {
        return fail(&format!("could not read stdin: {err}"), EXIT_FORMAT);
    }

    let formatted = match format::format(&input, &settings) {
        Ok(formatted) => formatted,
        Err(message) => return fail(&message, EXIT_FORMAT),
    };

    match std::io::stdout().write_all(formatted.as_bytes()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => fail(&format!("could not write stdout: {err}"), EXIT_FORMAT),
    }
}

fn fail(message: &str, code: u8) -> ExitCode {
    eprintln!("md-fmt: {message}");
    ExitCode::from(code)
}
