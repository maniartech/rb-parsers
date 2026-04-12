#![warn(missing_docs)]
//! Shared infrastructure for the Rust Parsers workspace.
//!
//! Provides spans, diagnostics, error catalog, suggestions, rendering,
//! hinting, recovery, and environment-detection primitives used by
//! `rb_tokenizer` and `rb_parser`.

/// Error catalog — machine-readable error codes and messages.
pub mod catalog;
/// Diagnostics — the `Diagnostic`, `DiagnosticBuilder`, and `Hint` types.
pub mod diagnostics;
/// Environment detection — determines the rendering environment at runtime.
pub mod env;
/// Automatic hinting — infers hints from diagnostic context.
pub mod hinting;
/// Recovery boundaries — defines where error recovery can restart.
pub mod recovery;
/// Renderers — formats diagnostics as human-readable or machine-readable output.
pub mod render;
/// Source spans and labels — `SourceSpan`, `SourcePosition`, and `SourceId`.
pub mod spans;
/// Suggestions and fixes — structured code-change proposals attached to diagnostics.
pub mod suggestions;

// Convenience re-exports for the most commonly needed span types.
pub use spans::{SourceId, SourceInfo, SourceRegistry};
pub use render::{RendererRegistry, RendererSelector};