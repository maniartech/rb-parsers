use crate::tokens::Token;
use crate::tokens::TokenizationError;

pub enum AcceptStrategy<'a> {
    None,
    StartChars(&'a str),
    Pattern(&'a str),
    Fn(Box<dyn Fn(&str) -> bool + 'a>),
}

pub trait Scanner {
    
    fn scan(&self, input: &str) -> Result<Option<Token>, TokenizationError>;
}
