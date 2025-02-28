use super::scanner::Scanner;
use crate::tokens::{Token, TokenizationError};

/// `BlockScanner` implementation for parsing block structures with start and end delimiters
/// that can be nested. This scanner handles structures like code blocks, comments blocks,
/// string literals with multi-character delimiters, etc.
pub struct BlockScanner {
    /// The start delimiter that marks the beginning of a block
    start_delimiter: String,

    /// The end delimiter that marks the end of a block
    end_delimiter: String,

    /// The token type to assign to matched blocks
    token_type: String,

    /// An optional token subtype for more specific categorization
    token_sub_type: Option<String>,

    /// Whether to support nested blocks with the same delimiters
    allow_nesting: bool,

    /// Whether to preserve the content exactly as-is (raw mode)
    /// When true, no escape sequence processing is performed
    raw_mode: bool,

    /// Whether to include the delimiters in the token value
    include_delimiters: bool,
}

impl BlockScanner {
    /// Creates a new block scanner with the specified delimiters and token type
    pub fn new(
        start_delimiter: &str,
        end_delimiter: &str,
        token_type: &str,
        token_sub_type: Option<&str>,
        allow_nesting: bool,
        raw_mode: bool,
        include_delimiters: bool,
    ) -> Self {
        Self {
            start_delimiter: start_delimiter.to_string(),
            end_delimiter: end_delimiter.to_string(),
            token_type: token_type.to_string(),
            token_sub_type: token_sub_type.map(|s| s.to_string()),
            allow_nesting,
            raw_mode,
            include_delimiters,
        }
    }

    /// Returns whether delimiters are included in the token value
    pub fn includes_delimiters(&self) -> bool {
        self.include_delimiters
    }

    /// Public method to find the end of a block from the input
    /// Returns the position after the end delimiter if found
    pub fn find_match_end(&self, input: &str) -> Result<Option<usize>, TokenizationError> {
        self.find_block_end(input)
    }

    /// Helper function to find the end delimiter position, handling nesting if enabled
    fn find_block_end(&self, input: &str) -> Result<Option<usize>, TokenizationError> {
        if !input.starts_with(&self.start_delimiter) {
            return Ok(None);
        }

        let mut position = self.start_delimiter.len();
        let mut nesting_level = 1;

        // Process characters until we find the matching end delimiter
        while position < input.len() {
            // Check for end delimiter
            if input[position..].starts_with(&self.end_delimiter) {
                nesting_level -= 1;
                if nesting_level == 0 {
                    return Ok(Some(position + self.end_delimiter.len()));
                }
                position += self.end_delimiter.len();
                continue;
            }

            // Check for nested start delimiter if nesting is allowed
            if self.allow_nesting && input[position..].starts_with(&self.start_delimiter) {
                nesting_level += 1;
                position += self.start_delimiter.len();
                continue;
            }

            // Handle escape sequences in non-raw mode
            if !self.raw_mode && position < input.len() - 1 && input[position..].starts_with('\\') {
                // Skip the escape character and the escaped character
                position += 2;
                continue;
            }

            // Move to next character (handles UTF-8 characters correctly)
            let next_char = input[position..].chars().next();
            if let Some(c) = next_char {
                position += c.len_utf8();
            } else {
                break;
            }
        }

        // If we get here, we didn't find a matching end delimiter
        Err(TokenizationError::UnmatchedBlockDelimiter(
            self.start_delimiter.clone(),
            self.end_delimiter.clone()
        ))
    }
}

impl Scanner for BlockScanner {
    fn scan(&self, input: &str) -> Result<Option<Token>, TokenizationError> {
        // Check if the input starts with the start delimiter
        if !input.starts_with(&self.start_delimiter) {
            return Ok(None);
        }

        // Find the end of the block
        match self.find_block_end(input) {
            Ok(Some(end_pos)) => {
                let full_match = &input[0..end_pos];

                let token_value = if self.include_delimiters {
                    full_match.to_string()
                } else {
                    input[self.start_delimiter.len()..end_pos - self.end_delimiter.len()].to_string()
                };

                // Create token with the correct length that accounts for delimiters
                // even when they're not included in the value
                let token = Token {
                    token_type: self.token_type.clone(),
                    token_sub_type: self.token_sub_type.clone(),
                    value: token_value,
                    line: 0,   // To be filled in by the tokenizer
                    column: 0, // To be filled in by the tokenizer
                };

                // Return the full match end position to ensure
                // tokenizer correctly advances past all consumed characters
                Ok(Some(token))
            },
            Ok(None) => Ok(None),
            Err(e) => Err(e),
        }
    }
}