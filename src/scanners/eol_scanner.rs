use super::scanner::Scanner;
use crate::tokens::{Token, TokenizationError};

/// `EolScanner` implementation for parsing structures that start with a specific delimiter
/// and continue until the end of line. This scanner handles structures like line comments,
/// preprocessor directives, and other line-oriented syntax.
pub struct EolScanner {
    /// The delimiter that marks the beginning of the line structure
    delimiter: String,

    /// The token type to assign to matched lines
    token_type: String,

    /// An optional token subtype for more specific categorization
    token_sub_type: Option<String>,

    /// Whether to include the delimiter in the token value
    include_delimiter: bool,
}

impl EolScanner {
    /// Creates a new EOL scanner with the specified delimiter and token type
    pub fn new(
        delimiter: &str,
        token_type: &str,
        token_sub_type: Option<&str>,
        include_delimiter: bool,
    ) -> Self {
        Self {
            delimiter: delimiter.to_string(),
            token_type: token_type.to_string(),
            token_sub_type: token_sub_type.map(|s| s.to_string()),
            include_delimiter,
        }
    }

    /// Returns whether the delimiter is included in the token value
    pub fn includes_delimiter(&self) -> bool {
        self.include_delimiter
    }

    /// Returns the delimiter string
    pub fn delimiter(&self) -> &str {
        &self.delimiter
    }

    /// Helper function to find the end of line position
    fn find_line_end(&self, input: &str) -> Option<usize> {
        // Check if the input starts with the delimiter
        if !input.starts_with(&self.delimiter) {
            return None;
        }

        // Find the next newline character
        let newline_pos = input.find('\n').unwrap_or(input.len());

        // Return the position after the newline, or the end of the input if no newline found
        if newline_pos < input.len() {
            Some(newline_pos + 1) // Include the newline in the match
        } else {
            Some(newline_pos) // End of input
        }
    }
}

impl Scanner for EolScanner {
    fn scan(&self, input: &str) -> Result<Option<Token>, TokenizationError> {
        // Check if the input starts with the delimiter
        if !input.starts_with(&self.delimiter) {
            return Ok(None);
        }

        // Find the end of line
        if let Some(end_pos) = self.find_line_end(input) {
            let full_match = &input[0..end_pos];

            let token_value = if self.include_delimiter {
                full_match.to_string()
            } else {
                input[self.delimiter.len()..end_pos].to_string()
            };

            // Create token with the correct value
            let token = Token {
                token_type: self.token_type.clone(),
                token_sub_type: self.token_sub_type.clone(),
                value: token_value,
                line: 0,   // To be filled in by the tokenizer
                column: 0, // To be filled in by the tokenizer
            };

            Ok(Some(token))
        } else {
            Ok(None)
        }
    }
}