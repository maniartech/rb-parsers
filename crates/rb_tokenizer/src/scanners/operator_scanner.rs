use super::scanner::Scanner;
use crate::tokens::{Token, TokenizationError, SourceSpan};
use std::borrow::Cow;

/// Matches multi-character (and single-character) operators using a
/// **longest-match-first** strategy with **no word-boundary check**.
///
/// This is the right choice for symbol-only operators like `++`, `+=`, `->`,
/// `=>`, `<<=`, `**`, `!=`, etc.  Unlike [`KeywordScanner`], it does not test
/// whether the character after the match is alphanumeric — that would produce
/// wrong results for operators that can appear directly adjacent to identifiers
/// (`x++`, `a->b`, `!cond`).
///
/// Operators are sorted by length at construction time so that a two-character
/// operator like `==` is always tested before the single-character `=`.
///
/// # Quick start
///
/// ```rust,ignore
/// use rb_tokenizer::scanners::OperatorScanner;
///
/// // All operators share one token_type; token_sub_type carries the exact operator
/// let scanner = OperatorScanner::with_subtypes("Op", &[
///     ("+=", "AddAssign"),
///     ("-=", "SubAssign"),
///     ("++", "Inc"),
///     ("--", "Dec"),
///     ("->", "Arrow"),
///     ("+",  "Add"),
///     ("-",  "Sub"),
///     ("=",  "Assign"),
/// ]);
/// ```
///
/// # Uniform token_type, no subtype
///
/// ```rust,ignore
/// let scanner = OperatorScanner::new("Operator", &["+=", "-=", "++", "--", "+", "-", "="]);
/// ```
///
/// [`KeywordScanner`]: crate::scanners::KeywordScanner
pub struct OperatorScanner {
    /// `(operator_text, token_sub_type)`, sorted longest-first.
    entries: Vec<(String, Option<&'static str>)>,
    token_type: &'static str,
}

impl OperatorScanner {
    /// All operators share `token_type` with no `token_sub_type`.
    ///
    /// The `operators` slice may be in any order; the scanner sorts
    /// longest-first internally.
    pub fn new(token_type: &'static str, operators: &[&str]) -> Self {
        let mut entries: Vec<(String, Option<&'static str>)> =
            operators.iter().map(|op| (op.to_string(), None)).collect();
        entries.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
        Self { entries, token_type }
    }

    /// Each operator gets a dedicated `token_sub_type`.
    ///
    /// The slice may be in any order; sorting is applied internally.
    ///
    /// ```rust,ignore
    /// OperatorScanner::with_subtypes("Op", &[
    ///     ("<<", "Shl"),
    ///     ("<",  "Lt"),
    ///     (">>", "Shr"),
    ///     (">",  "Gt"),
    /// ]);
    /// ```
    pub fn with_subtypes(token_type: &'static str, operators: &[(&str, &'static str)]) -> Self {
        let mut entries: Vec<(String, Option<&'static str>)> = operators
            .iter()
            .map(|(op, sub)| (op.to_string(), Some(*sub)))
            .collect();
        entries.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
        Self { entries, token_type }
    }
}

impl Scanner for OperatorScanner {
    fn scan<'i>(&self, input: &'i str) -> Result<Option<Token<'i>>, TokenizationError> {
        for (operator, sub_type) in &self.entries {
            if input.starts_with(operator.as_str()) {
                return Ok(Some(Token {
                    token_type: self.token_type,
                    token_sub_type: *sub_type,
                    value: Cow::Borrowed(&input[..operator.len()]),
                    span: SourceSpan::UNKNOWN,
                }));
            }
        }
        Ok(None)
    }
}
