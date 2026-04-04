use crate::tokens::Token;
use crate::tokens::TokenizationError;

pub struct ScanMatch {
    pub token: Token,
    pub consumed_len: usize,
}

pub enum AcceptStrategy {
    StartChars(&'static str),
    Pattern(&'static str),
    Fn(Box<dyn Fn(&str) -> bool + 'static>),
}

impl AcceptStrategy {
    pub fn accepts(&self, input: &str) -> bool {
        match self {
            AcceptStrategy::StartChars(chars) => input.chars().next().is_some_and(|c| chars.contains(c)),
            AcceptStrategy::Pattern(pat) => input.starts_with(pat),
            AcceptStrategy::Fn(f) => f(input),
        }
    }
}

pub trait Scanner {
    fn scan(&self, input: &str) -> Result<Option<Token>, TokenizationError>;

    fn scan_with_context(&self, input: &str) -> Result<Option<ScanMatch>, TokenizationError> {
        self.scan(input).map(|result| {
            result.map(|token| ScanMatch {
                consumed_len: token.value.len(),
                token,
            })
        })
    }
}
