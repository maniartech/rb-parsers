// ── Tokenizer error code catalog ─────────────────────────────────────────────

/// A typed, stable error code for a tokenization failure.
///
/// Codes are prefixed `"T_"` to distinguish them from parser codes (`"E_"`).
/// They are suitable as dictionary keys for error message localisation and as
/// stable identifiers in test assertions.
///
/// # Example
/// ```rust,ignore
/// if error.code() == catalog::UNMATCHED_INPUT {
///     // handle unrecognised input specifically
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TokenizationErrorCode(pub &'static str);

impl TokenizationErrorCode {
    /// Returns the error code as a `&'static str` identifier string.
    pub fn as_str(self) -> &'static str {
        self.0
    }
}

impl std::fmt::Display for TokenizationErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

// ── Well-known codes ──────────────────────────────────────────────────────────

/// No registered scanner matched at the current position.
pub const UNMATCHED_INPUT: TokenizationErrorCode    = TokenizationErrorCode("T_UNMATCHED_INPUT");

/// A character was encountered that no scanner accepts anywhere.
pub const UNEXPECTED_CHAR: TokenizationErrorCode    = TokenizationErrorCode("T_UNEXPECTED_CHAR");

/// A block-open token was found with no corresponding block-close.
pub const UNCLOSED_BLOCK: TokenizationErrorCode     = TokenizationErrorCode("T_UNCLOSED_BLOCK");

/// A regex pattern provided to a scanner was syntactically invalid.
pub const INVALID_REGEX: TokenizationErrorCode      = TokenizationErrorCode("T_INVALID_REGEX");

/// A scanner closure panicked or returned an internal error.
pub const SCANNER_PANIC: TokenizationErrorCode      = TokenizationErrorCode("T_SCANNER_PANIC");

/// A contextual scanner returned an error for a token that was context-dependent.
pub const CONTEXTUAL_MISMATCH: TokenizationErrorCode = TokenizationErrorCode("T_CONTEXTUAL_MISMATCH");
