use super::scanner::Scanner;
use crate::tokens::{Token, TokenizationError, SourceSpan};
use std::borrow::Cow;

/// A scanner that matches an exact literal symbol (e.g. `+`, `->`, `//`).
#[derive(Clone)]
pub struct SymbolScanner {
    /// The exact string this scanner matches.
    pub symbol: String,
    /// The token type label assigned on a match.
    pub token_type: &'static str,
    /// Optional sub-type for finer-grained classification.
    pub token_sub_type: Option<&'static str>,
}

impl SymbolScanner {
    /// Creates a `SymbolScanner` that matches `symbol` and labels tokens as `token_type`.
    pub fn new(symbol: &str, token_type: &'static str, token_sub_type: Option<&'static str>) -> Self {
        Self {
            symbol: symbol.to_string(),
            token_type,
            token_sub_type,
        }
    }
}

impl Scanner for SymbolScanner {
    fn first_bytes(&self) -> Option<Vec<u8>> {
        // A symbol always starts with its first byte.
        self.symbol.as_bytes().first().map(|&b| vec![b])
    }

    fn scan<'i>(&self, input: &'i str) -> Result<Option<Token<'i>>, TokenizationError> {
        if input.starts_with(&self.symbol) {
            Ok(Some(Token {
                span: SourceSpan::UNKNOWN,
                value: Cow::Borrowed(&input[..self.symbol.len()]),
                token_type: self.token_type,
                token_sub_type: self.token_sub_type,
            }))
        } else {
            Ok(None)
        }
    }
}
