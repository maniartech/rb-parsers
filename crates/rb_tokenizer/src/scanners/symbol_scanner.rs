use super::scanner::Scanner;
use crate::tokens::Token;
use crate::tokens::TokenizationError;

pub struct SymbolScanner {
    pub symbol: String,
    pub token_type: String,
    pub token_sub_type: Option<String>,
}

impl SymbolScanner {
    pub fn new(symbol: &str, token_type: &str, token_sub_type: Option<&str>) -> Self {
        Self {
            symbol: symbol.to_string(),
            token_type: token_type.to_string(),
            token_sub_type: token_sub_type.map(|s| s.to_string()),
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
                token_type: self.token_type.clone(),
                token_sub_type: self.token_sub_type.clone(),
            }))
        } else {
            Ok(None)
        }
    }
}
