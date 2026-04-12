use std::{error::Error, fmt};
use rb_common::spans::SourceSpan;
use crate::catalog::{self, TokenizationErrorCode};

/// Errors that may be produced during tokenization.
#[derive(Debug, Clone)]
pub enum TokenizationError {
    /// The input at the current position did not match any registered scanner.
    UnrecognizedToken(String),
    /// A block-delimited token was opened but never closed.
    UnmatchedBlockDelimiter(String, String),
    /// A regex-based scanner was constructed with an invalid regular expression.
    InvalidRegexPattern {
        /// The (possibly auto-anchored) pattern that failed to compile.
        pattern: String,
        /// The error message from the regex engine.
        message: String,
    },
    /// Any of the above errors enriched with a source position. Produced by
    /// the tokenizer loop when a scanner returns `Err(e)` and byte offset is known.
    WithSpan {
        /// The original error, without span information.
        error: Box<TokenizationError>,
        /// The source span at which the error occurred.
        span: SourceSpan,
    },
}

impl TokenizationError {
    /// Returns the typed error code for this variant.
    pub fn code(&self) -> TokenizationErrorCode {
        match self.inner() {
            TokenizationError::UnrecognizedToken(_)        => catalog::UNMATCHED_INPUT,
            TokenizationError::UnmatchedBlockDelimiter(..) => catalog::UNCLOSED_BLOCK,
            TokenizationError::InvalidRegexPattern { .. }  => catalog::INVALID_REGEX,
            TokenizationError::WithSpan { .. }             => catalog::UNMATCHED_INPUT, // unreachable via inner()
        }
    }

    /// Returns the span if this is a `WithSpan` variant, otherwise `None`.
    pub fn span(&self) -> Option<SourceSpan> {
        match self {
            TokenizationError::WithSpan { span, .. } => Some(*span),
            _ => None,
        }
    }

    /// Returns the innermost error, unwrapping any `WithSpan` wrapper.
    pub fn inner(&self) -> &TokenizationError {
        match self {
            TokenizationError::WithSpan { error, .. } => error.inner(),
            other => other,
        }
    }

    /// Attach a span to this error. If already `WithSpan`, the existing span is replaced.
    pub fn at(self, span: SourceSpan) -> Self {
        match self {
            TokenizationError::WithSpan { error, .. } => {
                TokenizationError::WithSpan { error, span }
            }
            err => TokenizationError::WithSpan { error: Box::new(err), span },
        }
    }
}

impl fmt::Display for TokenizationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TokenizationError::UnrecognizedToken(input) => {
                write!(f, "Unrecognized token: {}", input)
            },
            TokenizationError::UnmatchedBlockDelimiter(start, end) => {
                write!(f, "Unmatched block delimiter: start '{}' missing matching end '{}'", start, end)
            }
            TokenizationError::InvalidRegexPattern { pattern, message } => {
                write!(f, "Invalid regex pattern '{}': {}", pattern, message)
            }
            TokenizationError::WithSpan { error, span } => {
                write!(f, "{} (at byte {})", error, span.start.byte_offset)
            }
        }
    }
}

impl Error for TokenizationError {}
