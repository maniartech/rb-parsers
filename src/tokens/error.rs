use std::{error::Error, fmt};

#[derive(Debug)]
pub enum TokenizationError {
    UnrecognizedToken(String),
    // Define additional error types as needed.
}

impl fmt::Display for TokenizationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TokenizationError::UnrecognizedToken(input) => {
                write!(f, "Unrecognized token: {}", input)
            }
        }
    }
}

impl Error for TokenizationError {}
