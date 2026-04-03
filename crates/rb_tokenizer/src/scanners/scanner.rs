use crate::tokens::Token;
use crate::tokens::TokenizationError;

pub enum AcceptStrategy {
    StartChars(&'static str),
    Pattern(&'static str),
    Fn(Box<dyn Fn(&str) -> bool + 'static>),
}

impl AcceptStrategy {
    pub fn accepts(&self, input: &str) -> bool {
        match self {
            AcceptStrategy::StartChars(chars) => input.chars().next().map_or(false, |c| chars.contains(c)),
            AcceptStrategy::Pattern(pat) => input.starts_with(pat),
            AcceptStrategy::Fn(f) => f(input),
        }
    }
}

pub trait Scanner {
    fn scan(&self, input: &str) -> Result<Option<Token>, TokenizationError>;
}
