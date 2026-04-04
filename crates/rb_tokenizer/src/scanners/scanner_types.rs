use crate::tokens::{Token, TokenizationError};

use super::regex_scanner::RegexScanner;
use super::scanner::ScanMatch;
use super::symbol_scanner::SymbolScanner;
use super::block_scanner::BlockScanner;
use super::eol_scanner::EolScanner;
use super::{ClosureScanner, Scanner};

pub enum ScannerType {
    Symbol(SymbolScanner),
    Regex(RegexScanner),
    Block(BlockScanner),
    Eol(EolScanner),
    Closure(ClosureScanner),
    Scanner(Box<dyn Scanner>),
    Callback(Box<dyn CallbackScanner>),
}

pub trait CallbackScanner {
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

impl Scanner for ScannerType {
    fn scan(&self, input: &str) -> Result<Option<Token>, TokenizationError> {
        match self {
            ScannerType::Symbol(scanner) => scanner.scan(input),
            ScannerType::Regex(scanner) => scanner.scan(input),
            ScannerType::Block(scanner) => scanner.scan(input),
            ScannerType::Eol(scanner) => scanner.scan(input),
            ScannerType::Closure(scanner) => scanner.scan(input),
            ScannerType::Scanner(scanner) => scanner.scan(input),
            ScannerType::Callback(scanner) => scanner.scan(input),
        }
    }

    fn scan_with_context(&self, input: &str) -> Result<Option<ScanMatch>, TokenizationError> {
        match self {
            ScannerType::Symbol(scanner) => scanner.scan_with_context(input),
            ScannerType::Regex(scanner) => scanner.scan_with_context(input),
            ScannerType::Block(scanner) => scanner.scan_with_context(input),
            ScannerType::Eol(scanner) => scanner.scan_with_context(input),
            ScannerType::Closure(scanner) => scanner.scan_with_context(input),
            ScannerType::Scanner(scanner) => scanner.scan_with_context(input),
            ScannerType::Callback(scanner) => scanner.scan_with_context(input),
        }
    }
}
