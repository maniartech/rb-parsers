use crate::tokens::{Token, TokenizationError};

use super::regex_scanner::RegexScanner;
use super::symbol_scanner::SymbolScanner;
use super::block_scanner::BlockScanner;
use super::{ClosureScanner, Scanner};

pub enum ScannerType {
    Symbol(SymbolScanner),
    Regex(RegexScanner),
    Block(BlockScanner),
    Closure(ClosureScanner),
    Scanner(Box<dyn Scanner>),
    Callback(Box<dyn CallbackScanner>),
}

pub trait CallbackScanner {
    fn scan(&self, input: &str) -> Result<Option<Token>, TokenizationError>;
}

impl Scanner for ScannerType {
    fn scan(&self, input: &str) -> Result<Option<Token>, TokenizationError> {
        match self {
            ScannerType::Symbol(scanner) => scanner.scan(input),
            ScannerType::Regex(scanner) => scanner.scan(input),
            ScannerType::Block(scanner) => scanner.scan(input),
            ScannerType::Closure(scanner) => scanner.scan(input),
            ScannerType::Scanner(scanner) => scanner.scan(input),
            ScannerType::Callback(scanner) => scanner.scan(input),
        }
    }
}
