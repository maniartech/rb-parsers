use super::scanner::Scanner;
use crate::tokens::{Token, TokenizationError};

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Matches keywords (reserved words) with an automatic **word-boundary check**.
///
/// Unlike [`SymbolScanner`](crate::scanners::SymbolScanner), `KeywordScanner` refuses to
/// match when the character immediately following the keyword is a word character
/// (`[A-Za-z0-9_]`).  This prevents `if` from matching inside `ifdef`, `class` from
/// matching inside `classname`, etc.
///
/// Keywords are tried in **longest-first order** so `elsif` is preferred over `else`
/// when both are registered.
///
/// # Quick start
///
/// ```rust,ignore
/// use rb_tokenizer::scanners::KeywordScanner;
///
/// // All keywords share the same token_type, differentiated by token_sub_type
/// let scanner = KeywordScanner::with_subtypes("Keyword", &[
///     ("if",     "If"),
///     ("else",   "Else"),
///     ("elsif",  "Elsif"),
///     ("return", "Return"),
/// ]);
/// ```
///
/// # Custom word-boundary characters
///
/// For languages where `$` or `!` can appear in identifiers, override the default
/// word characters:
///
/// ```rust,ignore
/// let scanner = KeywordScanner::new("Keyword", &["def", "end"])
///     .with_word_boundary(|c| c.is_alphanumeric() || c == '_' || c == '?' || c == '!');
/// ```
pub struct KeywordScanner {
    /// `(keyword_text, token_sub_type)`, sorted longest-first.
    entries: Vec<(String, Option<&'static str>)>,
    token_type: &'static str,
    word_boundary: Box<dyn Fn(char) -> bool + Send + Sync>,
}

impl KeywordScanner {
    /// All keywords share `token_type` with no `token_sub_type`.
    pub fn new(token_type: &'static str, keywords: &[&str]) -> Self {
        let mut entries: Vec<(String, Option<&'static str>)> =
            keywords.iter().map(|k| (k.to_string(), None)).collect();
        entries.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
        Self {
            entries,
            token_type,
            word_boundary: Box::new(is_word_char),
        }
    }

    /// Each keyword gets a dedicated `token_sub_type`.
    ///
    /// ```rust,ignore
    /// KeywordScanner::with_subtypes("Keyword", &[
    ///     ("if",     "If"),
    ///     ("while",  "While"),
    ///     ("return", "Return"),
    /// ]);
    /// ```
    pub fn with_subtypes(token_type: &'static str, keywords: &[(&str, &'static str)]) -> Self {
        let mut entries: Vec<(String, Option<&'static str>)> = keywords
            .iter()
            .map(|(k, s)| (k.to_string(), Some(*s)))
            .collect();
        entries.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
        Self {
            entries,
            token_type,
            word_boundary: Box::new(is_word_char),
        }
    }

    /// Override the word-boundary predicate.
    ///
    /// The scanner rejects a keyword match when the character *immediately after* the
    /// keyword satisfies this function.  The default is `char::is_alphanumeric() || c == '_'`.
    pub fn with_word_boundary(mut self, pred: impl Fn(char) -> bool + Send + Sync + 'static) -> Self {
        self.word_boundary = Box::new(pred);
        self
    }
}

impl Scanner for KeywordScanner {
    fn scan(&self, input: &str) -> Result<Option<Token>, TokenizationError> {
        for (keyword, sub_type) in &self.entries {
            if !input.starts_with(keyword.as_str()) {
                continue;
            }
            // Word-boundary check: the char right after the keyword must NOT be a word char.
            let rest = &input[keyword.len()..];
            if rest.chars().next().is_some_and(|c| (self.word_boundary)(c)) {
                continue;
            }
            return Ok(Some(Token {
                token_type: self.token_type,
                token_sub_type: *sub_type,
                value: keyword.clone(),
                line: 0,
                column: 0,
            }));
        }
        Ok(None)
    }
}
