use super::scanner::Scanner;
use crate::tokens::{Token, TokenizationError, SourceSpan};
use std::borrow::Cow;

pub struct SymbolScanner {
    pub symbol: String,
    pub token_type: &'static str,
    pub token_sub_type: Option<&'static str>,
}

impl SymbolScanner {
    pub fn new(symbol: &str, token_type: &'static str, token_sub_type: Option<&'static str>) -> Self {
        Self {
            symbol: symbol.to_string(),
            token_type,
            token_sub_type,
        }
    }
}

impl Scanner for SymbolScanner {
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
