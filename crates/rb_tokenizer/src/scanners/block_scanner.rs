use super::scanner::Scanner;
use crate::tokens::{Token, TokenizationError};
use regex::Regex;
use std::collections::HashMap;

/// Types of escape rules supported by the scanner
pub enum EscapeRule {
    /// Simple single-character escape (e.g., \n, \t)
    Simple {
        /// Character that starts the escape (typically '\')
        escape_char: char,
    },

    /// Named escape sequence (e.g., &amp; in HTML)
    Named {
        /// Character that starts the escape
        start_char: char,
        /// Character that ends the escape
        end_char: char,
        /// Maximum length to look ahead (for performance)
        max_length: usize,
    },

    /// Regex-based escape pattern
    Pattern {
        /// Regular expression to match the escape sequence
        pattern: Regex,
    },

    /// Balanced escape like ${...} or \(...\)
    Balanced {
        /// Starting sequence
        start_seq: String,
        /// Ending sequence
        end_seq: String,
        /// Whether nested balanced escapes are allowed
        allow_nesting: bool,
    },
}

impl EscapeRule {
    /// Try to match an escape sequence at the current position
    /// Returns the length of the matched sequence or None if no match
    pub fn try_match(&self, input: &str, position: usize) -> Option<usize> {
        if position >= input.len() {
            return None;
        }

        match self {
            EscapeRule::Simple { escape_char: _ } => {
                // Process simple escape
                if position + 1 < input.len() {
                    return Some(2); // Escape character + the character it escapes
                }
                None
            },

            EscapeRule::Named { start_char, end_char, max_length } => {
                if input[position..].starts_with(*start_char) {
                    // Look for end_char within the max_length
                    let search_end = std::cmp::min(position + *max_length, input.len());
                    for i in position + 1..search_end {
                        if i < input.len() {
                            if let Some(c) = input[i..].chars().next() {
                                if c == *end_char {
                                    return Some(i - position + 1);
                                }
                            }
                        }
                    }
                }
                None
            },

            EscapeRule::Pattern { pattern } => {
                if let Some(mat) = pattern.find(&input[position..]) {
                    if mat.start() == 0 {  // Match must start at current position
                        return Some(mat.end());
                    }
                }
                None
            },

            EscapeRule::Balanced { start_seq, end_seq, allow_nesting } => {
                if position + start_seq.len() <= input.len() && input[position..].starts_with(start_seq) {
                    let mut pos = position + start_seq.len();
                    let mut nesting = 1;

                    while pos < input.len() {
                        if pos + end_seq.len() <= input.len() && input[pos..].starts_with(end_seq) {
                            nesting -= 1;
                            if nesting == 0 {
                                return Some(pos + end_seq.len() - position);
                            }
                            pos += end_seq.len();
                        } else if *allow_nesting && pos + start_seq.len() <= input.len() && input[pos..].starts_with(start_seq) {
                            nesting += 1;
                            pos += start_seq.len();
                        } else {
                            // Move to next character
                            let next_char = input[pos..].chars().next();
                            if let Some(c) = next_char {
                                pos += c.len_utf8();
                            } else {
                                break;
                            }
                        }
                    }
                }
                None
            },
        }
    }
}

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

    /// List of escape rules to apply
    escape_rules: Vec<EscapeRule>,

    /// Map for transforming escape sequences to their actual characters when processing token values
    escape_map: HashMap<String, char>,

    /// Whether to transform escaped sequences in the final token value
    transform_escapes: bool,
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
            escape_rules: Vec::new(),
            escape_map: HashMap::new(),
            transform_escapes: false,
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

    /// Add an escape rule to this scanner
    pub fn add_escape_rule(&mut self, rule: EscapeRule) {
        self.escape_rules.push(rule);
    }

    /// Add a simple escape character (typically backslash)
    pub fn add_simple_escape(&mut self, escape_char: char) {
        self.add_escape_rule(EscapeRule::Simple { escape_char });
    }

    /// Add a named escape sequence like HTML entities &...;
    pub fn add_named_escape(&mut self, start_char: char, end_char: char, max_length: usize) {
        self.add_escape_rule(EscapeRule::Named {
            start_char,
            end_char,
            max_length
        });
    }

    /// Add a regex pattern-based escape
    pub fn add_pattern_escape(&mut self, pattern: &str) -> Result<(), regex::Error> {
        let regex = Regex::new(pattern)?;
        self.add_escape_rule(EscapeRule::Pattern { pattern: regex });
        Ok(())
    }

    /// Add a balanced escape sequence like ${...} or \(...\)
    pub fn add_balanced_escape(&mut self, start_seq: &str, end_seq: &str, allow_nesting: bool) {
        self.add_escape_rule(EscapeRule::Balanced {
            start_seq: start_seq.to_string(),
            end_seq: end_seq.to_string(),
            allow_nesting
        });
    }

    /// Add a mapping from escape sequence to character for transformation
    pub fn add_escape_mapping(&mut self, sequence: &str, replacement: char) {
        self.escape_map.insert(sequence.to_string(), replacement);
    }

    /// Enable or disable escape transformation in token values
    pub fn set_transform_escapes(&mut self, transform: bool) {
        self.transform_escapes = transform;
    }

    /// Helper function to find the end delimiter position, handling nesting if enabled
    fn find_block_end(&self, input: &str) -> Result<Option<usize>, TokenizationError> {
        // Check if the input starts with the start delimiter
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
            if !self.raw_mode {
                let mut escape_handled = false;

                // Try each escape rule in order
                for rule in &self.escape_rules {
                    if let Some(len) = rule.try_match(input, position) {
                        // Skip over the entire escape sequence
                        position += len;
                        escape_handled = true;
                        break;
                    }
                }

                // If no escape rule matched but we still have a backslash,
                // use the default behavior (skip backslash and next character)
                if !escape_handled && position < input.len() - 1 && input[position..].starts_with('\\') {
                    // Skip the escape character and the escaped character
                    position += 2;
                    continue;
                }

                if escape_handled {
                    continue;
                }
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


    /// Process escape sequences in the token value
    fn process_escape_sequences(&self, input: &str) -> String {
        if !self.transform_escapes || self.raw_mode || self.escape_rules.is_empty() {
            return input.to_string();
        }

        let mut result = String::with_capacity(input.len());
        let mut chars = input.chars().peekable();

        while let Some(c) = chars.next() {
            // Handle simple escapes
            if let Some(EscapeRule::Simple { escape_char }) = self.escape_rules.iter().find(|r| matches!(r, EscapeRule::Simple { .. })) {
                if c == *escape_char {
                    if let Some(&next_c) = chars.peek() {
                        let escaped = next_c.to_string();
                        if let Some(&replacement) = self.escape_map.get(&escaped) {
                            result.push(replacement);
                        } else {
                            result.push(next_c);
                        }
                        chars.next();
                    }
                    continue;
                }
            }

            // Handle named escapes (e.g., &lt;)
            if let Some(EscapeRule::Named { start_char, end_char, max_length }) = self.escape_rules.iter().find(|r| matches!(r, EscapeRule::Named { .. })) {
                if c == *start_char {
                    // Try to collect up to max_length chars until end_char
                    let mut name = String::new();
                    let mut found = false;
                    for _ in 0..*max_length {
                        if let Some(&peek_c) = chars.peek() {
                            if peek_c == *end_char {
                                chars.next(); // consume end_char
                                found = true;
                                break;
                            } else {
                                name.push(peek_c);
                                chars.next();
                            }
                        } else {
                            break;
                        }
                    }
                    if found {
                        if let Some(&replacement) = self.escape_map.get(&name) {
                            result.push(replacement);
                            continue;
                        } else {
                            // Not a known escape, push as-is
                            result.push(*start_char);
                            result.push_str(&name);
                            result.push(*end_char);
                            continue;
                        }
                    } else {
                        // Not a valid escape, push as-is
                        result.push(*start_char);
                        result.push_str(&name);
                        continue;
                    }
                }
            }

            // Default: push character as-is
            result.push(c);
        }

        result
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

                let raw_value = if self.include_delimiters {
                    full_match.to_string()
                } else {
                    input[self.start_delimiter.len()..end_pos - self.end_delimiter.len()].to_string()
                };

                // Process escape sequences if needed
                let token_value = if !self.raw_mode && self.transform_escapes {
                    self.process_escape_sequences(&raw_value)
                } else {
                    raw_value
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