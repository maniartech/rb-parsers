use super::Scanner;
use crate::tokens::Token;
use crate::tokens::TokenizationError;
use super::scanner::AcceptStrategy;
use regex::Regex;

pub struct RegexScanner {
    pub pattern: Regex,
    pub token_type: String,
    pub token_sub_type: Option<String>,
    pub accept_strategy: Option<AcceptStrategy>,
}

impl RegexScanner {
    pub fn new(pattern: &str, token_type: &str, token_sub_type: Option<&str>) -> Self {
        Self {
            pattern: Regex::new(pattern).unwrap(),
            token_type: token_type.to_string(),
            token_sub_type: token_sub_type.map(|s| s.to_string()),
            accept_strategy: None,
        }
    }
    pub fn with_accept_strategy(pattern: &str, token_type: &str, token_sub_type: Option<&str>, accept_strategy: AcceptStrategy) -> Self {
        Self {
            pattern: Regex::new(pattern).unwrap(),
            token_type: token_type.to_string(),
            token_sub_type: token_sub_type.map(|s| s.to_string()),
            accept_strategy: Some(accept_strategy),
        }
    }
}

impl Scanner for RegexScanner {
    fn scan(&self, input: &str) -> Result<Option<Token>, TokenizationError> {
        if let Some(strategy) = &self.accept_strategy {
            if !strategy.accepts(input) {
                return Ok(None);
            }
        }
        if let Some(mat) = self.pattern.find(input) {
            return Ok(Some(Token {
                token_type: self.token_type.clone(),
                value: mat.as_str().to_string(),
                line: 0,
                column: 0,
                token_sub_type: None,
            }))
        }
        Ok(None)
    }
}
