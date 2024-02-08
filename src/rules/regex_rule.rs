use super::Rule;
use crate::tokens::Token;
use crate::tokens::TokenizationError;

use regex::Regex;

pub struct RegexRule {
    pub pattern: Regex,
    pub token_type: String,
    pub token_sub_type: Option<String>,
}

impl RegexRule {
    pub fn new(pattern: &str, token_type: &str, token_sub_type: Option<&str>) -> Self {
        Self {
            pattern: Regex::new(pattern).unwrap(),
            token_type: token_type.to_string(),
            token_sub_type: token_sub_type.map(|s| s.to_string()),
        }
    }
}

impl Rule for RegexRule {
    fn process(&self, input: &str) -> Result<Option<Token>, TokenizationError> {
        if let Some(mat) = self.pattern.find(input) {
            Ok(Some(Token {
                token_type: self.token_type.clone(),
                value: mat.as_str().to_string(),
                line: 0,
                column: 0,
                token_sub_type: None,
            }))
        } else {
            Ok(None)
        }
    }
}
