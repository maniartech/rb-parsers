use crate::tokens::Token;
use crate::tokens::TokenizationError;

// use regex::Regex;

pub trait Rule {
    fn process(&self, input: &str) -> Result<Option<Token>, TokenizationError>;
}

// struct RegexRule {
//     pattern: Regex,
//     token_type: String,
//     token_sub_type: Option<String>,
// }

// impl Rule for RegexRule {
//     fn process(&self, input: &str) -> Result<Option<Token>, TokenizationError> {
//         if let Some(mat) = self.pattern.find(input) {
//             Ok(Some(Token {
//                 line: 0,
//                 column: 0,
//                 value: mat.as_str().to_string(),
//                 token_type: self.token_type.clone(),
//                 token_sub_type: self.token_sub_type.clone(),
//             }))
//         } else {
//             Ok(None)
//         }
//     }
// }
