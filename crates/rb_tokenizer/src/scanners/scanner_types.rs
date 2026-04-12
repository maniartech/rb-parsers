use crate::tokens::{Token, TokenizationError};

use super::block_scanner::BlockScanner;
use super::char_class_scanner::CharClassScanner;
use super::contextual_scanner::ContextualScanner;
use super::eol_scanner::EolScanner;
use super::keyword_scanner::KeywordScanner;
use super::number_literal_scanner::NumberLiteralScanner;
use super::operator_scanner::OperatorScanner;
use super::regex_scanner::RegexScanner;
use super::scan_context::ScanContext;
use super::scanner::ScanMatch;
use super::symbol_scanner::SymbolScanner;
use super::whitespace_scanner::WhitespaceScanner;
use super::{ClosureScanner, Scanner};

pub enum ScannerType {
    Symbol(SymbolScanner),
    Regex(RegexScanner),
    Block(BlockScanner),
    Eol(EolScanner),
    Closure(ClosureScanner),
    Scanner(Box<dyn Scanner>),
    Callback(Box<dyn CallbackScanner>),
    /// Keywords with automatic word-boundary checking.
    Keyword(KeywordScanner),
    /// Character-class lead/continuation matching.
    CharClass(CharClassScanner),
    /// Configurable numeric literal scanner.
    NumberLiteral(NumberLiteralScanner),
    /// A scanner that receives mutable access to [`ScanContext`], enabling
    /// lexer-mode switching and context-sensitive tokenization.
    Contextual(Box<dyn ContextualScanner>),
    /// Longest-match operator scanner with no word-boundary check.
    Operator(OperatorScanner),
    /// Configurable whitespace scanner (uniform, split on newline, or with line continuation).
    Whitespace(WhitespaceScanner),
}

pub trait CallbackScanner {
    fn scan<'i>(&self, input: &'i str) -> Result<Option<Token<'i>>, TokenizationError>;

    fn scan_with_context<'i>(&self, input: &'i str) -> Result<Option<ScanMatch<'i>>, TokenizationError> {
        self.scan(input).map(|result| {
            result.map(|token| ScanMatch {
                consumed_len: token.value.len(),
                token,
            })
        })
    }
}

impl Scanner for ScannerType {
    fn scan<'i>(&self, input: &'i str) -> Result<Option<Token<'i>>, TokenizationError> {
        match self {
            ScannerType::Symbol(scanner)        => scanner.scan(input),
            ScannerType::Regex(scanner)         => scanner.scan(input),
            ScannerType::Block(scanner)         => scanner.scan(input),
            ScannerType::Eol(scanner)           => scanner.scan(input),
            ScannerType::Closure(scanner)       => scanner.scan(input),
            ScannerType::Scanner(scanner)       => scanner.scan(input),
            ScannerType::Callback(scanner)      => scanner.scan(input),
            ScannerType::Keyword(scanner)       => scanner.scan(input),
            ScannerType::CharClass(scanner)     => scanner.scan(input),
            ScannerType::NumberLiteral(scanner) => scanner.scan(input),
            ScannerType::Operator(scanner)       => scanner.scan(input),
            ScannerType::Whitespace(scanner)     => scanner.scan(input),
            // Contextual scanners can't be called without a ScanContext.
            // Use scan_contextually() instead.  Returning Ok(None) here
            // makes them invisible to the non-contextual tokenize() path.
            ScannerType::Contextual(_) => Ok(None),
        }
    }

    fn scan_with_context<'i>(&self, input: &'i str) -> Result<Option<ScanMatch<'i>>, TokenizationError> {
        match self {
            ScannerType::Symbol(scanner)        => scanner.scan_with_context(input),
            ScannerType::Regex(scanner)         => scanner.scan_with_context(input),
            ScannerType::Block(scanner)         => scanner.scan_with_context(input),
            ScannerType::Eol(scanner)           => scanner.scan_with_context(input),
            ScannerType::Closure(scanner)       => scanner.scan_with_context(input),
            ScannerType::Scanner(scanner)       => scanner.scan_with_context(input),
            ScannerType::Callback(scanner)      => scanner.scan_with_context(input),
            ScannerType::Keyword(scanner)       => scanner.scan_with_context(input),
            ScannerType::CharClass(scanner)     => scanner.scan_with_context(input),
            ScannerType::NumberLiteral(scanner) => scanner.scan_with_context(input),
            ScannerType::Operator(scanner)       => scanner.scan_with_context(input),
            ScannerType::Whitespace(scanner)     => scanner.scan_with_context(input),
            ScannerType::Contextual(_)          => Ok(None),
        }
    }
}

impl ScannerType {
    /// Returns the first-byte hint for this scanner variant by delegating to
    /// the inner scanner's [`Scanner::first_bytes`] implementation.
    /// Returns `None` for scanner types that cannot report a static first-byte set.
    pub fn first_bytes(&self) -> Option<Vec<u8>> {
        match self {
            ScannerType::Symbol(s)        => s.first_bytes(),
            ScannerType::Keyword(s)       => s.first_bytes(),
            ScannerType::Operator(s)      => s.first_bytes(),
            ScannerType::Block(s)         => s.first_bytes(),
            ScannerType::Eol(s)           => s.first_bytes(),
            ScannerType::NumberLiteral(s) => s.first_bytes(),
            ScannerType::Whitespace(s)    => s.first_bytes(),
            ScannerType::Regex(s)         => s.first_bytes(),
            ScannerType::CharClass(s)     => s.first_bytes(),
            ScannerType::Closure(s)       => s.first_bytes(),
            ScannerType::Scanner(s)       => s.first_bytes(),
            ScannerType::Callback(_)      => None,   // CallbackScanner has no first_bytes()
            ScannerType::Contextual(_)    => None,   // no context available at hint time
        }
    }

    /// Dispatch that supplies a [`ScanContext`] to [`Contextual`](ScannerType::Contextual)
    /// scanners, and falls back to the standard `scan_with_context` path for all others.
    ///
    /// Used by [`Tokenizer::tokenize_contextual`](crate::tokenizers::Tokenizer::tokenize_contextual).
    pub fn scan_contextually<'i>(
        &self,
        input: &'i str,
        ctx: &mut ScanContext,
    ) -> Result<Option<ScanMatch<'i>>, TokenizationError> {
        match self {
            ScannerType::Contextual(scanner) => scanner.scan_into_match(input, ctx),
            _ => self.scan_with_context(input),
        }
    }
}
