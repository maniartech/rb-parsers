use super::scanner::Scanner;
use crate::tokens::Token;
use crate::tokens::TokenizationError;

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
    fn scan(&self, input: &str) -> Result<Option<Token>, TokenizationError> {
        if input.starts_with(&self.symbol) {
            Ok(Some(Token {
                line: 0,
                column: 0,
                value: self.symbol.clone(),
                token_type: self.token_type,
                token_sub_type: self.token_sub_type,
            }))
        } else {
            Ok(None)
        }
    }
}
