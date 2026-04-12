use super::scanner::Scanner;
use super::scanner::ScanMatch;
use crate::tokens::{Token, TokenizationError, SourceSpan};
use std::borrow::Cow;

/// `EolScanner` implementation for parsing structures that start with a specific delimiter
/// and continue until the end of line. This scanner handles structures like line comments,
/// preprocessor directives, and other line-oriented syntax.
pub struct EolScanner {
    pub delimiter: String,
    pub token_type: &'static str,
    pub token_sub_type: Option<&'static str>,
    pub include_delimiter: bool,
}

impl EolScanner {
    pub fn new(
        delimiter: &str,
        token_type: &'static str,
        token_sub_type: Option<&'static str>,
        include_delimiter: bool,
    ) -> Self {
        Self {
            delimiter: delimiter.to_string(),
            token_type,
            token_sub_type,
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
    fn first_bytes(&self) -> Option<Vec<u8>> {
        // EOL token always starts with the first byte of the delimiter.
        self.delimiter.as_bytes().first().map(|&b| vec![b])
    }

    fn scan<'i>(&self, input: &'i str) -> Result<Option<Token<'i>>, TokenizationError> {
        // Check if the input starts with the delimiter
        if !input.starts_with(&self.delimiter) {
            return Ok(None);
        }

        // Find the end of line
        if let Some(end_pos) = self.find_line_end(input) {
            let token_value: Cow<'i, str> = if self.include_delimiter {
                Cow::Borrowed(&input[0..end_pos])
            } else {
                Cow::Borrowed(&input[self.delimiter.len()..end_pos])
            };

            // Create token with the correct value
            let token = Token {
                token_type: self.token_type,
                token_sub_type: self.token_sub_type,
                value: token_value,
                span: SourceSpan::UNKNOWN,
            };

            Ok(Some(token))
        } else {
            Ok(None)
        }
    }

    fn scan_with_context<'i>(&self, input: &'i str) -> Result<Option<ScanMatch<'i>>, TokenizationError> {
        if !input.starts_with(&self.delimiter) {
            return Ok(None);
        }

        if let Some(end_pos) = self.find_line_end(input) {
            let token_value: Cow<'i, str> = if self.include_delimiter {
                Cow::Borrowed(&input[0..end_pos])
            } else {
                Cow::Borrowed(&input[self.delimiter.len()..end_pos])
            };

            return Ok(Some(ScanMatch {
                consumed_len: end_pos,
                token: Token {
                    token_type: self.token_type,
                    token_sub_type: self.token_sub_type,
                    value: token_value,
                    span: SourceSpan::UNKNOWN,
                },
            }));
        }

        Ok(None)
    }
}