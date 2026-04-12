use crate::tokens::Token;
use crate::tokens::TokenizationError;

/// Result of a single scanner match, pairing the produced token with the number of bytes consumed.
pub struct ScanMatch<'src> {
    /// The token produced by the scanner.
    pub token: Token<'src>,
    /// The number of bytes consumed from the input — may differ from `token.value.len()`
    /// when the scanner strips delimiters, decodes escapes, or otherwise transforms
    /// the matched text before storing it in `value`.
    pub consumed_len: usize,
}

impl<'src> ScanMatch<'src> {
    /// Construct a `ScanMatch` where the token value exactly equals the consumed
    /// input fragment. Use this when `value` is an unmodified slice of `input`.
    pub fn verbatim(token: Token<'src>) -> Self {
        let consumed_len = token.value.len();
        ScanMatch { token, consumed_len }
    }

    /// Construct a `ScanMatch` with an explicit consumed length. Use this when
    /// the scanner's `value` differs from the consumed input — e.g. when delimiters
    /// are stripped or escape sequences are decoded.
    pub fn with_consumed(token: Token<'src>, consumed_len: usize) -> Self {
        ScanMatch { token, consumed_len }
    }
}

/// Optimisation hint that lets the `Tokenizer` quickly skip inapplicable scanners.
///
/// Checking this hint before calling `Scanner::scan` avoids full scanner
/// entry for the common case where the first byte rules out a match.
pub enum AcceptStrategy {
    /// Accept only when the first character of the input is in this charset.
    StartChars(&'static str),
    /// Accept only when the input starts with this literal pattern.
    Pattern(&'static str),
    /// Accept based on an arbitrary closure over the input.
    Fn(Box<dyn Fn(&str) -> bool + 'static>),
}

impl Clone for AcceptStrategy {
    fn clone(&self) -> Self {
        match self {
            // Data-free variants are trivially copyable.
            AcceptStrategy::StartChars(s) => AcceptStrategy::StartChars(s),
            AcceptStrategy::Pattern(s)    => AcceptStrategy::Pattern(s),
            // `Fn` closures are not Clone; we drop the hint. The scanner remains
            // semantically correct — it just loses the early-exit optimisation.
            AcceptStrategy::Fn(_) => AcceptStrategy::StartChars(""),
        }
    }
}

impl AcceptStrategy {
    /// Returns `true` when this strategy permits `input` to be passed to the scanner.
    pub fn accepts(&self, input: &str) -> bool {
        match self {
            AcceptStrategy::StartChars(chars) => input.chars().next().is_some_and(|c| chars.contains(c)),
            AcceptStrategy::Pattern(pat) => input.starts_with(pat),
            AcceptStrategy::Fn(f) => f(input),
        }
    }
}

/// Core scanner trait implemented by every token recogniser.
pub trait Scanner {
    /// Attempt to match the beginning of `input`.
    ///
    /// Returns `Ok(Some(token))` on success, `Ok(None)` when the scanner does
    /// not match, or `Err(e)` on a hard tokenization error.
    fn scan<'i>(&self, input: &'i str) -> Result<Option<Token<'i>>, TokenizationError>;

    /// Clone this scanner into a boxed trait object.
    ///
    /// The default implementation panics. Override this if your scanner type
    /// supports cloning (which most built-in scanners do).
    fn clone_box(&self) -> Box<dyn Scanner> {
        panic!("Scanner::clone_box() not implemented for this scanner type. \
               Override clone_box() in your Scanner implementation to support Tokenizer::clone().")
    }

    /// An optional hint listing the **first bytes** this scanner may match.
    ///
    /// Return `Some(bytes)` if the scanner can only ever begin a match when
    /// `input.as_bytes()[0]` is one of the listed values.  Return `None` if the
    /// scanner may match any starting byte (regex with variable prefix, catch-all
    /// recovery, etc.).
    ///
    /// This is called **once per registration** to build the tokenizer's first-byte
    /// dispatch table.  It is never called in the hot tokenize loop.
    ///
    /// The default returns `None` — the scanner participates in every dispatch.
    fn first_bytes(&self) -> Option<Vec<u8>> {
        None
    }

    /// Scan `input` and return a `ScanMatch` carrying both the token and the
    /// number of bytes that were actually consumed from `input`.
    ///
    /// # Contract
    ///
    /// The default implementation sets `consumed_len = token.value.len()`, which
    /// is **only correct** when the value is an unmodified slice of `input`.
    ///
    /// **Implementations MUST override this method** when `value` differs from the
    /// consumed bytes — for example, a scanner that strips delimiters or decodes
    /// escape sequences must set `consumed_len` to the actual number of bytes
    /// consumed from `input`, **not** the length of the transformed value.
    ///
    /// Failing to override in such cases will misalign the tokenizer cursor,
    /// causing subsequent scans to start at the wrong position.
    fn scan_with_context<'i>(&self, input: &'i str) -> Result<Option<ScanMatch<'i>>, TokenizationError> {
        self.scan(input).map(|result| {
            result.map(|token| ScanMatch::verbatim(token))
        })
    }
}

