use super::scanner::Scanner;
use crate::tokens::{Token, TokenizationError, SourceSpan};
use std::borrow::Cow;

// ── Internal char-class representation ───────────────────────────────────────

/// A compiled set of characters built from a concise spec string.
///
/// Spec syntax (mirrors the simplified form of regex character classes):
/// - `a-z`  — inclusive Unicode range from `a` to `z`
/// - `0-9`  — range
/// - `_`    — literal character
/// - `a-zA-Z0-9_` — combination of ranges and literals
///
/// A lone `-` (first or last character in the spec) is treated as a literal hyphen.
#[derive(Clone)]
struct CharClass {
    ranges:  Vec<(char, char)>,
    singles: Vec<char>,
}

impl CharClass {
    fn parse(spec: &str) -> Self {
        let chars: Vec<char> = spec.chars().collect();
        let mut ranges  = Vec::new();
        let mut singles = Vec::new();
        let mut i = 0;
        while i < chars.len() {
            // Treat '-' as a range operator only when it sits between two non-'-' chars
            // and is not at the first or last position.
            if i + 2 < chars.len() && chars[i + 1] == '-' && chars[i] != '-' && chars[i + 2] != '-' {
                let lo = chars[i];
                let hi = chars[i + 2];
                // Normalise so lo <= hi regardless of spec order.
                if lo <= hi {
                    ranges.push((lo, hi));
                } else {
                    ranges.push((hi, lo));
                }
                i += 3;
            } else {
                singles.push(chars[i]);
                i += 1;
            }
        }
        Self { ranges, singles }
    }

    #[inline]
    fn matches(&self, c: char) -> bool {
        self.singles.contains(&c)
            || self.ranges.iter().any(|&(lo, hi)| c >= lo && c <= hi)
    }
}

// ── Public scanner ────────────────────────────────────────────────────────────

/// Matches tokens defined by **lead** and **continuation** character classes,
/// without requiring a regular expression.
///
/// A token starts if the first input character satisfies the lead class, then
/// consumes as many additional characters as satisfy the continuation class.
/// If no continuation class is given, exactly one character is matched.
///
/// This covers identifiers, operators, and any repeating-char pattern that would
/// otherwise need a verbose regex.
///
/// # Spec syntax
///
/// Use simple range/literal notation: `"a-zA-Z_"`, `"0-9"`, `"a-z0-9_-"`.
///
/// # Built-in constructors
///
/// ```rust,ignore
/// CharClassScanner::identifier("Identifier", None);
///                         // → [a-zA-Z_][a-zA-Z0-9_]*
///
/// CharClassScanner::digits("Number", None);
///                         // → [0-9]+
/// ```
///
/// # Custom classes
///
/// ```rust,ignore
/// // Lisp symbols: first char [a-zA-Z+\-*/=<>!?], rest same + digits
/// CharClassScanner::new(
///     "a-zA-Z+-*/=<>!?",
///     Some("a-zA-Z0-9+-*/=<>!?"),
///     "Symbol",
///     None,
/// );
/// ```
#[derive(Clone)]
pub struct CharClassScanner {
    lead:         CharClass,
    continuation: Option<CharClass>,
    token_type:     &'static str,
    token_sub_type: Option<&'static str>,
}

impl CharClassScanner {
    /// Build from explicit spec strings.
    ///
    /// * `lead_spec` — character class for the first character (required).
    /// * `continuation_spec` — character class for subsequent characters.
    ///   `None` = match exactly one character.
    pub fn new(
        lead_spec: &str,
        continuation_spec: Option<&str>,
        token_type: &'static str,
        token_sub_type: Option<&'static str>,
    ) -> Self {
        Self {
            lead: CharClass::parse(lead_spec),
            continuation: continuation_spec.map(CharClass::parse),
            token_type,
            token_sub_type,
        }
    }

    /// Standard identifier: `[a-zA-Z_][a-zA-Z0-9_]*`
    pub fn identifier(token_type: &'static str, token_sub_type: Option<&'static str>) -> Self {
        Self::new("a-zA-Z_", Some("a-zA-Z0-9_"), token_type, token_sub_type)
    }

    /// Decimal digits: `[0-9]+`
    pub fn digits(token_type: &'static str, token_sub_type: Option<&'static str>) -> Self {
        Self::new("0-9", Some("0-9"), token_type, token_sub_type)
    }

    /// Hexadecimal digits: `[0-9a-fA-F]+`
    pub fn hex_digits(token_type: &'static str, token_sub_type: Option<&'static str>) -> Self {
        Self::new("0-9a-fA-F", Some("0-9a-fA-F"), token_type, token_sub_type)
    }
}

impl Scanner for CharClassScanner {
    fn scan<'i>(&self, input: &'i str) -> Result<Option<Token<'i>>, TokenizationError> {
        let mut chars = input.char_indices();

        // Check lead character.
        let (_, first_char) = match chars.next() {
            Some((pos, c)) if self.lead.matches(c) => (pos, c),
            _ => return Ok(None),
        };

        // Track exclusive end byte of the last accepted character.
        let mut byte_end = first_char.len_utf8();

        // Consume continuation characters.
        if let Some(cont) = &self.continuation {
            for (byte_pos, c) in chars {
                if cont.matches(c) {
                    byte_end = byte_pos + c.len_utf8();
                } else {
                    break;
                }
            }
        }

        Ok(Some(Token {
            token_type: self.token_type,
            token_sub_type: self.token_sub_type,
            value: Cow::Borrowed(&input[..byte_end]),
            span: SourceSpan::UNKNOWN,
        }))
    }
}
