use std::{error::Error, fmt};

#[derive(Debug, Clone)]
pub enum TokenizationError {
    UnrecognizedToken(String),
    UnmatchedBlockDelimiter(String, String),
    InvalidRegexPattern { pattern: String, message: String },
    // Define additional error types as needed.
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
        }
    }
}

impl Error for TokenizationError {}
