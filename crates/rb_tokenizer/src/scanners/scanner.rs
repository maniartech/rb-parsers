use crate::tokens::Token;
use crate::tokens::TokenizationError;

pub trait Scanner {
    fn scan(&self, input: &str) -> Result<Option<Token>, TokenizationError>;
}
