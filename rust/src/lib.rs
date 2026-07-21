//! The formatting engine behind md-fmt-rs.nvim, as a library so the integration
//! tests can call it without going through a subprocess.

pub mod cli;
pub mod format;
pub mod mdx;
pub mod table;
