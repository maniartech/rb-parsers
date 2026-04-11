use super::scanner::Scanner;
use crate::tokens::{Token, TokenizationError, SourceSpan};
use std::borrow::Cow;

/// A declarative, configurable scanner for numeric literals.
///
/// Enable each format you need via the builder flags; everything else is left
/// off by default.  [`NumberLiteralScanner::new`] is a batteries-included
/// constructor that enables all common formats for most languages (C, Rust, Python,
/// JavaScript, etc.).  Use [`NumberLiteralScanner::minimal`] to start with only
/// plain decimal integers and opt-in explicitly.
///
/// # Supported formats
///
/// | Flag | Example |
/// |------|---------|
/// | `allow_float` | `1.5`, `1.`, `.5` (with `allow_leading_dot`) |
/// | `allow_hex` | `0xFF`, `0XDEADBEEF` |
/// | `allow_binary` | `0b1010`, `0B1010` |
/// | `allow_octal` | `0o777`, `0O777` |
/// | `allow_scientific` | `1e10`, `1.5E-3`, `6.02e+23` |
/// | `allow_underscores` | `1_000_000`, `0xFF_FF` |
/// | `allow_leading_dot` | `.5`, `.75e2` |
///
/// # Examples
///
/// ```rust,ignore
/// use rb_tokenizer::scanners::NumberLiteralScanner;
///
/// // Full-featured (Rust/Python/JS style)
/// let scanner = NumberLiteralScanner::new("Number", None);
///
/// // Minimal: integers only (e.g. assembly hex/dec)
/// let scanner = NumberLiteralScanner::minimal("Number", None)
///     .allow_hex(true);
///
/// // JSON numbers
/// let scanner = NumberLiteralScanner::minimal("Number", None)
///     .allow_float(true)
///     .allow_scientific(true)
///     .allow_leading_dot(true);
/// ```
pub struct NumberLiteralScanner {
    pub token_type:     &'static str,
    pub token_sub_type: Option<&'static str>,

    /// Allow decimal point and fractional part (e.g. `1.5`).
    pub allow_float: bool,
    /// Allow `0x` / `0X` hexadecimal prefix.
    pub allow_hex: bool,
    /// Allow `0b` / `0B` binary prefix.
    pub allow_binary: bool,
    /// Allow `0o` / `0O` octal prefix.
    pub allow_octal: bool,
    /// Allow `e` / `E` scientific-notation suffix (e.g. `1e10`, `1.5e-3`).
    pub allow_scientific: bool,
    /// Allow `_` as a digit-group separator (e.g. `1_000_000`).
    pub allow_underscores: bool,
    /// Allow a literal starting with `.` (e.g. `.5` → `0.5`).
    pub allow_leading_dot: bool,
}

impl NumberLiteralScanner {
    /// All common formats enabled (hex, binary, octal, float, scientific, underscores).
    /// Leading-dot floats (`.5`) are **disabled** by default — enable with
    /// `.allow_leading_dot(true)` if your language supports them.
    pub fn new(token_type: &'static str, token_sub_type: Option<&'static str>) -> Self {
        Self {
            token_type,
            token_sub_type,
            allow_float: true,
            allow_hex: true,
            allow_binary: true,
            allow_octal: true,
            allow_scientific: true,
            allow_underscores: true,
            allow_leading_dot: false,
        }
    }

    /// Only plain decimal integers enabled.  Opt in to additional formats with
    /// the fluent builder methods below.
    pub fn minimal(token_type: &'static str, token_sub_type: Option<&'static str>) -> Self {
        Self {
            token_type,
            token_sub_type,
            allow_float: false,
            allow_hex: false,
            allow_binary: false,
            allow_octal: false,
            allow_scientific: false,
            allow_underscores: false,
            allow_leading_dot: false,
        }
    }

    // ── Builder ───────────────────────────────────────────────────────────────

    pub fn allow_float(mut self, v: bool) -> Self        { self.allow_float        = v; self }
    pub fn allow_hex(mut self, v: bool) -> Self          { self.allow_hex          = v; self }
    pub fn allow_binary(mut self, v: bool) -> Self       { self.allow_binary       = v; self }
    pub fn allow_octal(mut self, v: bool) -> Self        { self.allow_octal        = v; self }
    pub fn allow_scientific(mut self, v: bool) -> Self   { self.allow_scientific   = v; self }
    pub fn allow_underscores(mut self, v: bool) -> Self  { self.allow_underscores  = v; self }
    pub fn allow_leading_dot(mut self, v: bool) -> Self  { self.allow_leading_dot  = v; self }
}

// ── Parse helpers ─────────────────────────────────────────────────────────────

/// Consume a run of `[0-9_]` (possibly restricting to a narrower digit set `is_digit`),
/// returning the consumed slice length.  An underscore is only accepted when
/// `allow_underscores` is true AND it is between two valid digit characters (not at
/// start or end).
fn consume_digits<F: Fn(char) -> bool>(
    input: &[u8],
    offset: usize,
    is_digit: F,
    allow_underscores: bool,
) -> usize {
    let mut pos = offset;
    let mut last_was_sep = false;

    while pos < input.len() {
        let c = input[pos] as char;
        if is_digit(c) {
            pos += 1;
            last_was_sep = false;
        } else if allow_underscores && c == '_' && !last_was_sep && pos > offset {
            // Peek ahead: underscore is only valid when followed by a digit.
            if pos + 1 < input.len() && is_digit(input[pos + 1] as char) {
                pos += 1;
                last_was_sep = true;
            } else {
                break;
            }
        } else {
            break;
        }
    }
    pos
}

impl Scanner for NumberLiteralScanner {
    fn scan<'i>(&self, input: &'i str) -> Result<Option<Token<'i>>, TokenizationError> {
        let bytes = input.as_bytes();
        if bytes.is_empty() {
            return Ok(None);
        }

        // ── Leading-dot float: `.5`, `.75e2` ─────────────────────────────────
        if self.allow_float && self.allow_leading_dot && bytes[0] == b'.' {
            if bytes.len() < 2 || !bytes[1].is_ascii_digit() {
                return Ok(None);
            }
            let mut pos = 1; // skip '.'
            pos = consume_digits(bytes, pos, |c| c.is_ascii_digit(), self.allow_underscores);

            // Optional scientific part.
            if self.allow_scientific && pos < bytes.len() {
                let e = bytes[pos] as char;
                if e == 'e' || e == 'E' {
                    let mut ep = pos + 1;
                    if ep < bytes.len() && (bytes[ep] == b'+' || bytes[ep] == b'-') {
                        ep += 1;
                    }
                    let after = consume_digits(bytes, ep, |c| c.is_ascii_digit(), self.allow_underscores);
                    if after > ep {
                        pos = after;
                    }
                }
            }

            return Ok(Some(Token {
                token_type: self.token_type,
                token_sub_type: self.token_sub_type,
                value: Cow::Borrowed(&input[..pos]),
                span: SourceSpan::UNKNOWN,
            }));
        }

        // ── Must start with a digit for all remaining forms ───────────────────
        if !bytes[0].is_ascii_digit() {
            return Ok(None);
        }

        // ── Based literals: 0x, 0b, 0o ───────────────────────────────────────
        if bytes[0] == b'0' && bytes.len() >= 2 {
            match bytes[1] as char {
                'x' | 'X' if self.allow_hex => {
                    let start = 2;
                    let end = consume_digits(
                        bytes, start,
                        |c| c.is_ascii_hexdigit(),
                        self.allow_underscores,
                    );
                    if end > start {
                        return Ok(Some(Token {
                            token_type: self.token_type,
                            token_sub_type: self.token_sub_type,
                            value: Cow::Borrowed(&input[..end]),
                            span: SourceSpan::UNKNOWN,
                        }));
                    }
                }
                'b' | 'B' if self.allow_binary => {
                    let start = 2;
                    let end = consume_digits(
                        bytes, start,
                        |c| c == '0' || c == '1',
                        self.allow_underscores,
                    );
                    if end > start {
                        return Ok(Some(Token {
                            token_type: self.token_type,
                            token_sub_type: self.token_sub_type,
                            value: Cow::Borrowed(&input[..end]),
                            span: SourceSpan::UNKNOWN,
                        }));
                    }
                }
                'o' | 'O' if self.allow_octal => {
                    let start = 2;
                    let end = consume_digits(
                        bytes, start,
                        |c| ('0'..='7').contains(&c),
                        self.allow_underscores,
                    );
                    if end > start {
                        return Ok(Some(Token {
                            token_type: self.token_type,
                            token_sub_type: self.token_sub_type,
                            value: Cow::Borrowed(&input[..end]),
                            span: SourceSpan::UNKNOWN,
                        }));
                    }
                }
                _ => {}
            }
        }

        // ── Decimal integer (possibly with float / scientific suffix) ─────────
        let mut pos = consume_digits(bytes, 0, |c| c.is_ascii_digit(), self.allow_underscores);
        if pos == 0 {
            return Ok(None);
        }

        // Optional fractional part.
        let mut is_float = false;
        if self.allow_float && pos < bytes.len() && bytes[pos] == b'.' {
            // Peek ahead: must be followed by a digit (avoids consuming `.` in `1..2`).
            let after_dot = pos + 1;
            if after_dot < bytes.len() && bytes[after_dot].is_ascii_digit() {
                pos = after_dot;
                pos = consume_digits(bytes, pos, |c| c.is_ascii_digit(), self.allow_underscores);
                is_float = true;
            } else if after_dot == bytes.len() {
                // Trailing dot: `1.` — consume if float is enabled.
                pos = after_dot;
                is_float = true;
            }
        }

        // Optional scientific part.
        if self.allow_scientific && pos < bytes.len() {
            let e = bytes[pos] as char;
            if e == 'e' || e == 'E' {
                let mut ep = pos + 1;
                if ep < bytes.len() && (bytes[ep] == b'+' || bytes[ep] == b'-') {
                    ep += 1;
                }
                let after = consume_digits(bytes, ep, |c| c.is_ascii_digit(), self.allow_underscores);
                if after > ep {
                    pos = after;
                    is_float = true;
                }
            }
        }

        let _ = is_float; // available for sub_type logic if callers want it

        Ok(Some(Token {
            token_type: self.token_type,
            token_sub_type: self.token_sub_type,
            value: Cow::Borrowed(&input[..pos]),
            span: SourceSpan::UNKNOWN,
        }))
    }
}
