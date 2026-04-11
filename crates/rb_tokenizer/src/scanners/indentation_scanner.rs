use std::sync::Mutex;

use super::contextual_scanner::ContextualScanner;
use super::scan_context::ScanContext;
use super::scanner::ScanMatch;
use crate::tokens::{Token, TokenizationError, SourceSpan};
use std::borrow::Cow;

// ── Internal state ────────────────────────────────────────────────────────────

struct IndentState {
    /// Stack of indentation column widths.  The bottom element is always `0`.
    stack: Vec<usize>,
}

impl Default for IndentState {
    fn default() -> Self {
        Self { stack: vec![0] }
    }
}

// ── Public scanner ────────────────────────────────────────────────────────────

/// A **contextual** scanner that emits `INDENT` and `DEDENT` tokens for
/// languages with significant whitespace (Python, YAML, CoffeeScript, Haskell
/// layout, Makefile recipe blocks, etc.).
///
/// ## How it works
///
/// `IndentationScanner` fires **only at the start of a line** (`ctx.column == 1`).
/// It counts leading spaces or tabs, compares against the indent-level stack, and:
///
/// * **Same level** — returns `Ok(None)`; the tokenizer's built-in whitespace
///   skipping consumes the leading spaces normally.
/// * **Deeper** — emits a single `INDENT` token whose value is the leading
///   whitespace string, and pushes the new level onto the stack.
/// * **Shallower** — emits a single `DEDENT` token whose value is the decimal
///   string representation of *how many levels were popped* (e.g. `"2"`), and
///   pops entries off the stack.  Consumed bytes equal the actual whitespace, so
///   the tokenizer advances correctly.
///
/// > Because one scanner invocation = one token, DEDENT by N levels emits a
/// > single token with `value = "N"`.  Your parser should treat that as N
/// > implicit close-blocks.
///
/// ## Requirements
///
/// * Must be used with [`Tokenizer::tokenize_contextual`](crate::tokenizers::Tokenizer::tokenize_contextual)
///   (requires `ctx.column` tracking).
/// * The tokenizer's `tokenize_whitespace` option must be **disabled** (the
///   default) so that same-level leading whitespace is skipped normally without
///   emitting spurious Whitespace tokens.
/// * Register with [`Tokenizer::add_contextual_scanner`](crate::tokenizers::Tokenizer::add_contextual_scanner);
///   put it **before** other scanners so it claims the leading whitespace first.
///
/// ## Example
///
/// ```rust,ignore
/// use rb_tokenizer::{Tokenizer, scanners::IndentationScanner};
///
/// let mut tokenizer = Tokenizer::new();
/// // IndentationScanner goes first
/// tokenizer.add_contextual_scanner(Box::new(IndentationScanner::new("Indent", "Dedent")));
/// tokenizer.add_keyword_scanner("Keyword", &["if", "else", "for", "def"]);
/// tokenizer.add_char_class_scanner("a-zA-Z_", Some("a-zA-Z0-9_"), "Identifier", None);
/// // ...
///
/// let tokens = tokenizer.tokenize_contextual("def foo:\n    return 1\n").unwrap();
/// ```
pub struct IndentationScanner {
    /// Token type emitted when the indentation level increases.
    pub indent_token_type: &'static str,
    /// Token type emitted when the indentation level decreases.
    pub dedent_token_type: &'static str,
    state: Mutex<IndentState>,
}

impl IndentationScanner {
    pub fn new(indent_token_type: &'static str, dedent_token_type: &'static str) -> Self {
        Self {
            indent_token_type,
            dedent_token_type,
            state: Mutex::new(IndentState::default()),
        }
    }

    /// Reset the indentation stack to `[0]`.
    ///
    /// Call this between tokenization runs if you reuse the same scanner instance.
    pub fn reset(&self) {
        *self.state.lock().unwrap() = IndentState::default();
    }

    /// Measure the leading whitespace in `input`, returning `(column_width, byte_len)`.
    ///
    /// Spaces count as 1 each.  Tabs count as 1 each (single-column model —
    /// consistent with most modern tooling).  Scanning stops at the first
    /// non-whitespace character or at EOF.
    fn measure_indent(input: &str) -> (usize, usize) {
        let mut cols  = 0usize;
        let mut bytes = 0usize;
        for c in input.chars() {
            match c {
                ' '  => { cols += 1; bytes += 1; }
                '\t' => { cols += 1; bytes += 1; }
                _    => break,
            }
        }
        (cols, bytes)
    }
}

impl ContextualScanner for IndentationScanner {
    fn scan<'i>(&self, input: &'i str, ctx: &mut ScanContext) -> Result<Option<Token<'i>>, TokenizationError> {
        // scan_into_match is the primary override; scan is not used directly.
        // Provide a no-op implementation to satisfy the trait.
        let _ = (input, ctx);
        Ok(None)
    }

    /// Primary override — called by [`ScannerType::scan_contextually`].
    fn scan_into_match<'i>(&self, input: &'i str, ctx: &mut ScanContext) -> Result<Option<ScanMatch<'i>>, TokenizationError> {
        // Only fire at the start of a line.
        if ctx.column != 1 {
            return Ok(None);
        }

        let (indent_cols, indent_bytes) = Self::measure_indent(input);

        // Skip blank / whitespace-only lines — don't emit INDENT/DEDENT for them.
        let rest = &input[indent_bytes..];
        if rest.is_empty() || rest.starts_with('\n') || rest.starts_with('\r') {
            return Ok(None);
        }

        let mut state = self.state.lock().unwrap();
        let top = *state.stack.last().unwrap_or(&0);

        if indent_cols > top {
            // ── INDENT ───────────────────────────────────────────────────────
            state.stack.push(indent_cols);
            let token = Token {
                token_type: self.indent_token_type,
                token_sub_type: None,
                value: Cow::Borrowed(&input[..indent_bytes]),
                span: SourceSpan::UNKNOWN,
            };
            return Ok(Some(ScanMatch { consumed_len: indent_bytes, token }));
        }

        if indent_cols < top {
            // ── DEDENT ───────────────────────────────────────────────────────
            let mut levels = 0usize;
            while let Some(&l) = state.stack.last() {
                if l <= indent_cols {
                    break;
                }
                state.stack.pop();
                levels += 1;
            }
            let token = Token {
                token_type: self.dedent_token_type,
                token_sub_type: None,
                // Value encodes the number of levels popped so the parser can
                // know how many scopes to close in one shot.
                value: Cow::Owned(levels.to_string()),
                span: SourceSpan::UNKNOWN,
            };
            // Consume the leading whitespace even though the value differs in length.
            return Ok(Some(ScanMatch { consumed_len: indent_bytes, token }));
        }

        // Same level — let the tokenizer's whitespace-skipping handle the spaces.
        Ok(None)
    }
}
