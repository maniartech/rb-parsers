use super::Scanner;

use crate::tokens::Token;
use crate::tokens::TokenizationError;

/// The function signature accepted by `ClosureScanner`.
pub type ScanClosure = dyn for<'i> Fn(&'i str) -> Result<Option<Token<'i>>, TokenizationError>;

/// A scanner backed by an arbitrary boxed closure.
///
/// Useful for quick one-off scanners in tests or prototypes where defining a
/// named type would be overkill.
pub struct ClosureScanner {
    // cb is a closure that takes a string slice and returns a Result<Option<Token>, TokenizationError>
    cb: Box<ScanClosure>,
}

impl Clone for ClosureScanner {
    fn clone(&self) -> Self {
        panic!("ClosureScanner cannot be cloned because it wraps a non-Clone closure. \
               Use a named type that implements Clone, or wrap it in an Arc.")
    }
}

impl ClosureScanner {
    /// Creates a `ClosureScanner` wrapping the given boxed closure.
    pub fn new(cb: Box<ScanClosure>) -> Self {
        ClosureScanner { cb }
    }
}

impl Scanner for ClosureScanner {
    fn scan<'i>(&self, input: &'i str) -> Result<Option<Token<'i>>, TokenizationError> {
        (self.cb)(input)
    }
}
