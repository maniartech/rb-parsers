use super::Rule;

use crate::tokens::Token;
use crate::tokens::TokenizationError;

pub struct ClosureRule {
    // cb is a closure that takes a string slice and returns a Result<Option<Token>, TokenizationError>
    cb: Box<dyn Fn(&str) -> Result<Option<Token>, TokenizationError>>,
}

impl ClosureRule {
    pub fn new(cb: Box<dyn Fn(&str) -> Result<Option<Token>, TokenizationError>>) -> Self {
        ClosureRule { cb }
    }
}

impl Rule for ClosureRule {
    fn process(&self, input: &str) -> Result<Option<Token>, TokenizationError> {
        (self.cb)(input)
    }
}
