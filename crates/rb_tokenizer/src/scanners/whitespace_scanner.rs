use super::scanner::Scanner;
use crate::tokens::{Token, TokenizationError, SourceSpan};
use std::borrow::Cow;

/// Tokenizes whitespace with configurable treatment of newlines and line
/// continuations.
///
/// Three ready-made constructors cover the most common language families:
///
/// | Mode | Constructor | Use for |
/// |---|---|---|
/// | Uniform | [`WhitespaceScanner::uniform`] | All whitespace as one token. C, Java, JSON, SQL, Lua. |
/// | Split | [`WhitespaceScanner::split`] | Separate tokens for newlines and horizontal whitespace. Go, JavaScript, Kotlin, Swift, Ruby (ASI). |
/// | Continuation | [`WhitespaceScanner::with_continuation`] | Split + `\<newline>` as line-continuation. C preprocessor, Python `\`, Bash, Make. |
///
/// # Why a dedicated scanner?
///
/// `add_regex_scanner(r"^\s+", "Whitespace", None)` treats all whitespace
/// identically.  That is not sufficient when:
///
/// - **ASI languages** (JavaScript, Go, Swift, Kotlin, Ruby) need newlines as
///   distinct tokens so the parser can decide where semicolons are inserted.
/// - **Line-continuation languages** (C macros, Python `\`, Bash) treat a
///   backslash immediately before a newline as a line-splice, not as an operator.
/// - The grammar needs to distinguish blank lines from a single newline, or
///   `\r\n` from `\n` for source-span accuracy.
///
/// # Indentation-sensitive languages
///
/// For Python-/YAML-/Haskell-style INDENT/DEDENT tokens use
/// [`IndentationScanner`](crate::scanners::IndentationScanner) instead — it
/// already handles significant whitespace at the token level.
///
/// # Newline normalisation
///
/// All three forms — `\n`, `\r\n`, and bare `\r` — are recognised as one
/// logical newline boundary.  The original bytes are preserved in
/// `Token::value`.
#[derive(Clone)]
pub struct WhitespaceScanner {
    /// Token type for horizontal whitespace (` `, `\t`).
    ///
    /// In **uniform mode** (`newline_token_type` is `None`) this covers all
    /// whitespace including newlines.
    pub token_type: &'static str,

    /// If `Some`, newlines (`\n`, `\r\n`, `\r`) emit a separate token of this
    /// type (**split mode**).  If `None` all whitespace is uniform.
    pub newline_token_type: Option<&'static str>,

    /// If `Some`, a backslash immediately followed by a newline emits a token
    /// of this type instead of being tokenized as a `\` operator + newline.
    ///
    /// This is only checked when a backslash appears in the input; it does not
    /// affect any other character.  Requires split mode — consider using
    /// [`with_continuation`](Self::with_continuation) which sets both.
    pub continuation_token_type: Option<&'static str>,
}

impl WhitespaceScanner {
    /// **Uniform mode** — all whitespace (spaces, tabs, newlines) is emitted as
    /// a single token type.
    ///
    /// Best for languages that treat all whitespace as ignorable trivia:
    /// C, C++, Java, JSON, SQL, Lua, TOML.
    ///
    /// ```rust,ignore
    /// t.add_whitespace_scanner(WhitespaceScanner::uniform("Whitespace"));
    /// ```
    pub fn uniform(token_type: &'static str) -> Self {
        Self {
            token_type,
            newline_token_type: None,
            continuation_token_type: None,
        }
    }

    /// **Split mode** — horizontal whitespace and newlines produce separate
    /// token types.
    ///
    /// Best for languages with automatic semicolon insertion (ASI) or where
    /// newlines terminate statements:
    /// **Go**, **JavaScript**, **TypeScript**, **Kotlin**, **Swift**, **Ruby**,
    /// **CoffeeScript**, **Python** (at statement level).
    ///
    /// ```rust,ignore
    /// t.add_whitespace_scanner(WhitespaceScanner::split("Whitespace", "Newline"));
    /// ```
    pub fn split(token_type: &'static str, newline_token_type: &'static str) -> Self {
        Self {
            token_type,
            newline_token_type: Some(newline_token_type),
            continuation_token_type: None,
        }
    }

    /// **Continuation mode** — like [`split`](Self::split) but also recognises
    /// a backslash immediately before a newline as a line-continuation token.
    ///
    /// Best for:
    /// - **C / C++ preprocessor** macros: `#define LONG_MACRO \` + newline
    /// - **Python**: explicit line continuation with `\`
    /// - **Bash / shell scripts**: `\` at end of line continues the command
    /// - **GNU Make** recipe lines
    ///
    /// ```rust,ignore
    /// t.add_whitespace_scanner(WhitespaceScanner::with_continuation(
    ///     "Whitespace",
    ///     "Newline",
    ///     "LineContinuation",
    /// ));
    /// ```
    pub fn with_continuation(
        token_type: &'static str,
        newline_token_type: &'static str,
        continuation_token_type: &'static str,
    ) -> Self {
        Self {
            token_type,
            newline_token_type: Some(newline_token_type),
            continuation_token_type: Some(continuation_token_type),
        }
    }
}

impl Scanner for WhitespaceScanner {
    fn first_bytes(&self) -> Option<Vec<u8>> {
        // Whitespace scanners always consume ASCII whitespace.
        // In split mode: space, tab (horizontal only).
        // In uniform mode: all ASCII whitespace.
        // Line-continuation mode also accepts backslash.
        let split_mode = self.newline_token_type.is_some();
        let mut bytes: Vec<u8> = if split_mode {
            vec![b' ', b'\t', b'\n', b'\r']  // split still handles \n/\r in its newline branch
        } else {
            vec![b' ', b'\t', b'\n', b'\r', 0x0B, 0x0C]
        };
        if self.continuation_token_type.is_some() {
            bytes.push(b'\\');
        }
        bytes.sort_unstable();
        bytes.dedup();
        Some(bytes)
    }

    fn scan<'i>(&self, input: &'i str) -> Result<Option<Token<'i>>, TokenizationError> {
        let first = match input.chars().next() {
            None => return Ok(None),
            Some(c) => c,
        };

        // ── 1. Line continuation: backslash immediately before a newline ───────
        if let Some(cont_type) = self.continuation_token_type {
            if first == '\\' {
                let after_backslash = &input[1..];
                let nl_len: Option<usize> = if after_backslash.starts_with("\r\n") {
                    Some(2)
                } else if after_backslash.starts_with('\n') || after_backslash.starts_with('\r') {
                    Some(1)
                } else {
                    None
                };
                if let Some(nl) = nl_len {
                    let total = 1 + nl;
                    return Ok(Some(Token {
                        token_type: cont_type,
                        token_sub_type: None,
                        value: Cow::Borrowed(&input[..total]),
                        span: SourceSpan::UNKNOWN,
                    }));
                }
                // Backslash not followed by newline — not a line continuation;
                // fall through so another scanner handles it.
                return Ok(None);
            }
        }

        // ── 2. Newline (split mode only) ──────────────────────────────────────
        if let Some(nl_type) = self.newline_token_type {
            if first == '\r' || first == '\n' {
                let len = if first == '\r' && input.starts_with("\r\n") {
                    2
                } else {
                    1
                };
                return Ok(Some(Token {
                    token_type: nl_type,
                    token_sub_type: None,
                    value: Cow::Borrowed(&input[..len]),
                    span: SourceSpan::UNKNOWN,
                }));
            }
        }

        // ── 3. Horizontal whitespace (or all whitespace in uniform mode) ──────
        //
        // In split mode stop at the first newline character so the next call
        // can emit a Newline token.  In uniform mode consume everything that
        // Rust considers whitespace.
        let split_mode = self.newline_token_type.is_some();
        let is_consumable = |c: char| -> bool {
            if split_mode {
                c == ' ' || c == '\t'
            } else {
                c.is_whitespace()
            }
        };

        if !is_consumable(first) {
            return Ok(None);
        }

        let len: usize = input
            .chars()
            .take_while(|&c| is_consumable(c))
            .map(|c| c.len_utf8())
            .sum();

        Ok(Some(Token {
            token_type: self.token_type,
            token_sub_type: None,
            value: Cow::Borrowed(&input[..len]),
            span: SourceSpan::UNKNOWN,
        }))
    }
}
