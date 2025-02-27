use crate::tokens::Token;
use crate::tokens::TokenizationError;

// use regex::Regex;

pub trait Rule {
    fn process(&self, input: &str) -> Result<Option<Token>, TokenizationError>;
}
