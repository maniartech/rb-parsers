use crate::tokens::Token;
use crate::tokens::TokenizationError;

pub struct ScanMatch<'src> {
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
    fn scan<'i>(&self, input: &'i str) -> Result<Option<Token<'i>>, TokenizationError>;

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

