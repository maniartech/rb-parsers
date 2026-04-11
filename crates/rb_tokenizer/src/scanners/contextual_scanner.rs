use crate::tokens::{Token, TokenizationError};
use super::scan_context::ScanContext;
use super::scanner::ScanMatch;

/// A scanner that receives mutable access to [`ScanContext`], enabling lexer-mode
/// switching and context-dependent tokenization decisions.
///
/// Register implementations with
/// [`Tokenizer::add_contextual_scanner`](crate::tokenizers::Tokenizer::add_contextual_scanner)
/// or [`Tokenizer::add_contextual_closure`](crate::tokenizers::Tokenizer::add_contextual_closure).
///
/// # Implementing lexer modes
///
/// The canonical use case is string interpolation (e.g. `f"hello {name}"`).
/// Each scanner reads `ctx.mode` to decide what to match, and writes `ctx.mode`
/// to switch the tokenizer into a different scanning state for the next token:
///
/// ```rust,ignore
/// use rb_tokenizer::scanners::{ContextualScanner, ScanContext};
/// use rb_tokenizer::tokens::{Token, TokenizationError};
///
/// pub const MODE_NORMAL:    u32 = 0;
/// pub const MODE_IN_STRING: u32 = 1;
/// pub const MODE_IN_INTERP: u32 = 2;
///
/// pub struct StringModeScanner;
///
/// impl ContextualScanner for StringModeScanner {
///     fn scan(&self, input: &str, ctx: &mut ScanContext) -> Result<Option<Token>, TokenizationError> {
///         match ctx.mode {
///             MODE_NORMAL => {
///                 if input.starts_with('"') {
///                     ctx.mode = MODE_IN_STRING;
///                     // return the opening-quote token ...
///                 }
///                 Ok(None)
///             }
///             MODE_IN_STRING => {
///                 // scan string body; on '{' set ctx.mode = MODE_IN_INTERP
///                 Ok(None)
///             }
///             MODE_IN_INTERP => {
///                 // scan interpolated expression; on '}' set ctx.mode = MODE_IN_STRING
///                 Ok(None)
///             }
///             _ => Ok(None),
///         }
///     }
/// }
/// ```
///
/// # Context-sensitive scanning
///
/// Scanners can also inspect `ctx.line`, `ctx.column`, and `ctx.prev_token_kind`
/// for position- or history-dependent decisions — for example, Python-style
/// significant indentation.
pub trait ContextualScanner: Send + Sync {
    fn scan<'i>(&self, input: &'i str, ctx: &mut ScanContext) -> Result<Option<Token<'i>>, TokenizationError>;

    /// Like `scan` but returns a [`ScanMatch`] that separates the emitted [`Token`]
    /// from the number of bytes consumed from `input`.
    ///
    /// The default implementation sets `consumed_len = token.value.len()`, which is
    /// correct for the vast majority of scanners.  Override this when the bytes you
    /// want to consume differ from the token value — the canonical case is
    /// [`IndentationScanner`](crate::scanners::IndentationScanner), which needs to
    /// consume the leading whitespace bytes while emitting `DEDENT "N"` (a 1-byte
    /// value string encoding the number of levels popped).
    fn scan_into_match<'i>(&self, input: &'i str, ctx: &mut ScanContext) -> Result<Option<ScanMatch<'i>>, TokenizationError> {
        self.scan(input, ctx).map(|r| r.map(|token| ScanMatch {
            consumed_len: token.value.len(),
            token,
        }))
    }
}

/// Wraps a closure into a [`ContextualScanner`].
///
/// The closure receives the remaining input slice and a mutable reference to
/// [`ScanContext`].  See [`ContextualScanner`] for usage examples.
type ContextualScannerFn =
    dyn for<'i> Fn(&'i str, &mut ScanContext) -> Result<Option<Token<'i>>, TokenizationError> + Send + Sync;

pub struct ContextualClosureScanner {
    cb: Box<ContextualScannerFn>,
}

impl ContextualClosureScanner {
    pub fn new(
        cb: impl for<'i> Fn(&'i str, &mut ScanContext) -> Result<Option<Token<'i>>, TokenizationError>
            + Send
            + Sync
            + 'static,
    ) -> Self {
        Self { cb: Box::new(cb) }
    }
}

impl ContextualScanner for ContextualClosureScanner {
    fn scan<'i>(&self, input: &'i str, ctx: &mut ScanContext) -> Result<Option<Token<'i>>, TokenizationError> {
        (self.cb)(input, ctx)
    }
}

