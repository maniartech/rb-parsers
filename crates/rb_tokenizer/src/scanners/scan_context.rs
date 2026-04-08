/// Mutable context passed through the tokenizer loop to context-aware scanners.
///
/// Enables two key capabilities:
///
/// * **Lexer modes** — scanners can read and write [`mode`](Self::mode) to switch between
///   distinct scanning states (e.g. normal code, inside a string, inside an interpolated
///   expression like `"hello {name}"`).
///
/// * **Context-sensitive scanning** — scanners can inspect [`line`](Self::line),
///   [`column`](Self::column), and [`prev_token_kind`](Self::prev_token_kind) to make
///   position- or history-dependent decisions (e.g. Python indentation tracking).
///
/// # Lexer mode conventions
///
/// Define your modes as `u32` constants or a `#[repr(u32)] enum`:
///
/// ```rust,ignore
/// const MODE_NORMAL:        u32 = 0;
/// const MODE_IN_STRING:     u32 = 1;
/// const MODE_IN_INTERP:     u32 = 2;
/// ```
///
/// The tokenizer initializes `mode` to `0` (normal) and updates `line`, `column`,
/// and `prev_token_kind` after each successfully emitted token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanContext {
    /// User-defined lexer mode identifier.  `0` = "normal" mode.
    /// Write a new value here to switch the tokenizer state for the next scan attempt.
    pub mode: u32,

    /// Current line number (1-based) at the start of this scan attempt.
    pub line: usize,

    /// Current column number (1-based) at the start of this scan attempt.
    pub column: usize,

    /// The `token_type` of the most recently emitted token, or `None` before the
    /// first token has been produced.
    pub prev_token_kind: Option<&'static str>,
}

impl Default for ScanContext {
    fn default() -> Self {
        Self {
            mode: 0,
            line: 1,
            column: 1,
            prev_token_kind: None,
        }
    }
}

impl ScanContext {
    pub fn new() -> Self {
        Self::default()
    }
}
