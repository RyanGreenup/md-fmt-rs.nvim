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
    md-fmt --version
    md-fmt --help

Options:
    --mdx                Parse MDX constructs and preserve them verbatim.
    --width N            Wrap prose at N columns. 0 disables wrapping.
                         Default: 80.
    --frontmatter DELIM  Treat DELIM as the frontmatter fence. Default: ---
    --no-frontmatter     Do not recognize frontmatter.
";

pub enum Outcome {
    /// Format stdin with these settings.
    Run(Settings),
    /// Print this on stdout and exit successfully.
    Message(String),
}

pub fn parse(args: &[String]) -> Result<Outcome, String> {
    let mut settings = Settings::default();
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
            _ => return Err(format!("unknown argument `{arg}`\n\n{USAGE}")),
        }
        i += 1;
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
