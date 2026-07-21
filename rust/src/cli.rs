//! Argument parsing.
//!
//! Hand-rolled rather than delegating to clap. The plugin compiles this crate
//! on the user's machine the first time they format a buffer, so every
//! dependency is paid for in wall-clock time that the user watches.

use crate::format::Settings;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub const USAGE: &str = "\
md-fmt - format Markdown and MDX on stdin, write the result to stdout

Usage:
    md-fmt [--mdx] [--width N] [--frontmatter DELIM | --no-frontmatter]
    md-fmt --table --row N --col N
    md-fmt --version
    md-fmt --help

Options:
    --mdx                Parse MDX constructs and preserve them verbatim.
    --width N            Wrap prose at N columns. 0 disables wrapping.
                         Default: 80.
    --frontmatter DELIM  Treat DELIM as the frontmatter fence. Default: ---
    --no-frontmatter     Do not recognize frontmatter.
    --table              Realign only the GFM table under the cursor, leaving
                         the rest of the document alone. Prints the line range
                         to replace, where the cursor ends up, and the
                         replacement:

                             FIRST LAST CURSOR_ROW CURSOR_COL
                             <one line per line of FIRST..=LAST>

                         Prints nothing and exits non-zero when the cursor is
                         not in a table.
    --row N              1-based cursor line, as from nvim_win_get_cursor.
    --col N              0-based byte cursor column, as from
                         nvim_win_get_cursor.
";

pub enum Outcome {
    /// Format stdin with these settings.
    Run(Settings),
    /// Realign the table under this cursor position. The settings come along
    /// because the document has to be parsed the same way either command
    /// would parse it.
    Table {
        settings: Settings,
        row: usize,
        col: usize,
    },
    /// Print this on stdout and exit successfully.
    Message(String),
}

pub fn parse(args: &[String]) -> Result<Outcome, String> {
    let mut settings = Settings::default();
    let mut table = false;
    let mut row = None;
    let mut col = None;
    let mut i = 0;

    while i < args.len() {
        let arg = args[i].as_str();
        match arg {
            "--help" | "-h" => return Ok(Outcome::Message(USAGE.to_string())),
            "--version" | "-V" => {
                return Ok(Outcome::Message(format!("md-fmt {VERSION}\n")));
            }
            "--mdx" => settings.mdx = true,
            "--no-frontmatter" => settings.frontmatter = None,
            "--table" => table = true,
            "--width" => {
                let value = next(args, &mut i, "--width")?;
                settings.width = value
                    .parse()
                    .map_err(|_| format!("--width expects a number, got `{value}`"))?;
            }
            "--frontmatter" => {
                let value = next(args, &mut i, "--frontmatter")?;
                settings.frontmatter = Some(value.to_string());
            }
            "--row" => {
                let value = next(args, &mut i, "--row")?;
                row = Some(
                    value
                        .parse()
                        .map_err(|_| format!("--row expects a number, got `{value}`"))?,
                );
            }
            "--col" => {
                let value = next(args, &mut i, "--col")?;
                col = Some(
                    value
                        .parse()
                        .map_err(|_| format!("--col expects a number, got `{value}`"))?,
                );
            }
            _ => return Err(format!("unknown argument `{arg}`\n\n{USAGE}")),
        }
        i += 1;
    }

    if table {
        let row = row.ok_or_else(|| format!("--table needs --row\n\n{USAGE}"))?;
        let col = col.ok_or_else(|| format!("--table needs --col\n\n{USAGE}"))?;
        return Ok(Outcome::Table { settings, row, col });
    }

    Ok(Outcome::Run(settings))
}

/// Consume the value that follows a flag, advancing the index past it.
fn next<'a>(args: &'a [String], i: &mut usize, flag: &str) -> Result<&'a str, String> {
    *i += 1;
    args.get(*i)
        .map(String::as_str)
        .ok_or_else(|| format!("{flag} expects a value"))
}
