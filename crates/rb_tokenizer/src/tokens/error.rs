use std::{error::Error, fmt};
use rb_common::spans::SourceSpan;

#[derive(Debug, Clone)]
pub enum TokenizationError {
    UnrecognizedToken(String),
    UnmatchedBlockDelimiter(String, String),
    InvalidRegexPattern { pattern: String, message: String },
    /// Any of the above errors enriched with a source position. Produced by
    /// the tokenizer loop when a scanner returns `Err(e)` and byte offset is known.
    WithSpan { error: Box<TokenizationError>, span: SourceSpan },
}

impl TokenizationError {
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
