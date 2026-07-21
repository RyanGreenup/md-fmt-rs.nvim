//! The formatting engine behind mdfmt.nvim, as a library so the integration
//! tests can call it without going through a subprocess.

pub mod cli;
pub mod format;
pub mod mdx;
