/// Tokenization error type.
pub mod error;
/// `Token` struct and associated helpers.
pub mod token;

pub use error::TokenizationError;
pub use token::Token;

// Re-export span types so scanner files can use crate::tokens::{SourceSpan, SourceId, SourcePosition}
pub use rb_common::spans::{SourceId, SourcePosition, SourceSpan};
