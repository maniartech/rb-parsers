//! Shared infrastructure for the Rust Parsers workspace.
//!
//! Provides spans, diagnostics, error catalog, suggestions, rendering,
//! hinting, recovery, and environment-detection primitives used by
//! `rb_tokenizer` and `rb_parser`.

pub mod catalog;
pub mod diagnostics;
pub mod env;
pub mod hinting;
pub mod recovery;
pub mod render;
pub mod spans;
pub mod suggestions;