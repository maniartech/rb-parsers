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

/// The concrete scanner variant stored in `Tokenizer::scanners`.
///
/// Each arm wraps a specific scanner implementation. The `Tokenizer` dispatches
/// to the appropriate variant based on the first-byte dispatch table.
pub enum ScannerType {
    /// A literal symbol scanner (`SymbolScanner`).
    Symbol(SymbolScanner),
    /// A regex-backed scanner (`RegexScanner`).
    Regex(RegexScanner),
    /// A delimited block scanner (`BlockScanner`).
    Block(BlockScanner),
    /// An end-of-line scanner (`EolScanner`).
    Eol(EolScanner),
    /// A closure-backed scanner (`ClosureScanner`).
    Closure(ClosureScanner),
    /// An arbitrary boxed `Scanner` implementation.
    Scanner(Box<dyn Scanner>),
    /// An arbitrary boxed `CallbackScanner` implementation.
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

/// A scanner variant that receives the raw input string and returns an optional token.
///
/// Unlike [`Scanner`], this trait has no access to context. It is used when you need
/// a simple callback with full control over token construction.
pub trait CallbackScanner {
    /// Scans `input` and returns the first matching token, or `None` if no match.
    fn scan<'i>(&self, input: &'i str) -> Result<Option<Token<'i>>, TokenizationError>;

    /// Like `scan` but returns a [`ScanMatch`] carrying the exact consumed byte length.
    fn scan_with_context<'i>(&self, input: &'i str) -> Result<Option<ScanMatch<'i>>, TokenizationError> {
        self.scan(input).map(|result| {
            result.map(|token| ScanMatch {
                consumed_len: token.value.len(),
                token,
            })
        })
    }

    /// Clone into a boxed trait object. Panics by default.
    fn clone_box(&self) -> Box<dyn CallbackScanner> {
        panic!("CallbackScanner::clone_box() not implemented. \
               Override it to support Tokenizer::clone().")
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

impl Clone for ScannerType {
    fn clone(&self) -> Self {
        match self {
            ScannerType::Symbol(s)        => ScannerType::Symbol(s.clone()),
            ScannerType::Regex(s)         => ScannerType::Regex(s.clone()),
            ScannerType::Block(s)         => ScannerType::Block(s.clone()),
            ScannerType::Eol(s)           => ScannerType::Eol(s.clone()),
            ScannerType::Closure(s)       => ScannerType::Closure(s.clone()),
            ScannerType::Keyword(s)       => ScannerType::Keyword(s.clone()),
            ScannerType::CharClass(s)     => ScannerType::CharClass(s.clone()),
            ScannerType::NumberLiteral(s) => ScannerType::NumberLiteral(s.clone()),
            ScannerType::Operator(s)      => ScannerType::Operator(s.clone()),
            ScannerType::Whitespace(s)    => ScannerType::Whitespace(s.clone()),
            // `Box<dyn Scanner/CallbackScanner/ContextualScanner>` — delegate to clone_box().
            // This panics at runtime if the concrete type hasn't overridden clone_box().
            ScannerType::Scanner(s)    => ScannerType::Scanner(s.clone_box()),
            ScannerType::Callback(s)   => ScannerType::Callback(s.clone_box()),
            ScannerType::Contextual(s) => ScannerType::Contextual(s.clone_box()),
        }
    }
}
