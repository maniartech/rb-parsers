use super::Scanner;

use crate::tokens::Token;
use crate::tokens::TokenizationError;

pub struct ClosureScanner {
    // cb is a closure that takes a string slice and returns a Result<Option<Token>, TokenizationError>
    cb: Box<dyn Fn(&str) -> Result<Option<Token>, TokenizationError>>,
}

impl ClosureScanner {
    pub fn new(cb: Box<dyn Fn(&str) -> Result<Option<Token>, TokenizationError>>) -> Self {
        ClosureScanner { cb }
    }
}

impl Scanner for ClosureScanner {
    fn scan(&self, input: &str) -> Result<Option<Token>, TokenizationError> {
        (self.cb)(input)
    }
}
