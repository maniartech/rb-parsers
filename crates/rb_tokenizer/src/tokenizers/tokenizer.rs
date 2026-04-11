use crate::scanners::char_class_scanner::CharClassScanner;
use crate::scanners::contextual_scanner::{ContextualClosureScanner, ContextualScanner};
use crate::scanners::keyword_scanner::KeywordScanner;
use crate::scanners::number_literal_scanner::NumberLiteralScanner;
use crate::scanners::operator_scanner::OperatorScanner;
use crate::scanners::scan_context::ScanContext;
use crate::scanners::whitespace_scanner::WhitespaceScanner;
use crate::scanners::{self, BlockScanner, EolScanner, RegexScanner, Scanner, ScannerType, SymbolScanner};
use crate::tokens::{Token, TokenizationError};
use rb_common::spans::{SourceId, SourcePosition, SourceSpan};
use std::borrow::Cow;
use std::cell::RefCell;

#[derive(Debug, Clone)]
pub struct TokenizerConfig {
    pub tokenize_whitespace: bool,
    pub continue_on_error: bool,
    pub error_tolerance_limit: usize,
    pub track_token_positions: bool,        // Controls whether line/column tracking is performed
}

impl Default for TokenizerConfig {
    fn default() -> Self {
        Self {
            tokenize_whitespace: false,
            continue_on_error: false,
            error_tolerance_limit: 10,
            track_token_positions: true,     // Default to tracking positions
        }
    }
}

pub struct Tokenizer {
    scanners: Vec<ScannerType>,
    config: TokenizerConfig,
    last_errors: RefCell<Option<Vec<TokenizationError>>>,
    /// Source identity used on all tokens produced by this instance.
    pub source_id: SourceId,
}

impl Default for Tokenizer {
    fn default() -> Self {
        Self::new()
    }
}

impl Tokenizer {
    /// Advance line/column counters by scanning the bytes of a consumed token slice.
    /// Counts Unicode code points (non-continuation bytes) for column tracking.
    #[inline]
    fn advance_pos(slice: &[u8], current_line: &mut usize, current_column: &mut usize) {
        for &b in slice {
            if b == b'\n' {
                *current_line += 1;
                *current_column = 1;
            } else if b & 0xC0 != 0x80 {
                // Non-continuation byte = start of a Unicode code point.
                *current_column += 1;
            }
        }
    }

    pub fn new() -> Self {
        Tokenizer {
            scanners: Vec::new(),
            config: TokenizerConfig::default(),
            last_errors: RefCell::new(None),
            source_id: SourceId::UNKNOWN,
        }
    }

    pub fn with_config(config: TokenizerConfig) -> Self {
        Tokenizer {
            scanners: Vec::new(),
            config,
            last_errors: RefCell::new(None),
            source_id: SourceId::UNKNOWN,
        }
    }

    /// Sets the source identity used on all tokens produced by this instance.
    pub fn with_source_id(mut self, id: SourceId) -> Self {
        self.source_id = id;
        self
    }

    pub fn config(&self) -> &TokenizerConfig {
        &self.config
    }

    pub fn config_mut(&mut self) -> &mut TokenizerConfig {
        &mut self.config
    }

    /// Returns any errors encountered during the last tokenization operation
    pub fn last_errors(&self) -> Option<Vec<TokenizationError>> {
        self.last_errors.borrow().clone()
    }

    pub fn add_scanner(&mut self, scanner: Box<dyn scanners::Scanner>) {
        self.scanners.push(ScannerType::Scanner(scanner));
    }

    pub fn add_scanner_with_priority(&mut self, scanner: Box<dyn scanners::Scanner>, priority: usize) {
        // Insert scanner at the specified priority (lower index = higher priority)
        if priority >= self.scanners.len() {
            self.scanners.push(ScannerType::Scanner(scanner));
        } else {
            self.scanners.insert(priority, ScannerType::Scanner(scanner));
        }
    }

    pub fn add_regex_scanner(
        &mut self,
        pattern: &str,
        token_type: &'static str,
        sub_token_type: Option<&'static str>,
    ) -> Result<&mut Self, TokenizationError> {
        let scanner = ScannerType::Regex(RegexScanner::new(pattern, token_type, sub_token_type)?);
        self.scanners.push(scanner);
        Ok(self)
    }

    pub fn add_symbol_scanner(&mut self, symbol: &str, token_type: &'static str, default_scanner: Option<&'static str>) {
        let scanner = ScannerType::Symbol(SymbolScanner::new(symbol, token_type, default_scanner));
        self.scanners.push(scanner);
    }

    pub fn add_closure_scanner(
        &mut self,
        cb: Box<scanners::closure_scanner::ScanClosure>,
    ) {
        let scanner = ScannerType::Closure(scanners::ClosureScanner::new(cb));
        self.scanners.push(scanner);
    }

    pub fn add_callback_scanner(&mut self, cb: Box<dyn scanners::CallbackScanner>) {
        let scanner = ScannerType::Callback(cb);
        self.scanners.push(scanner);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn add_block_scanner(
        &mut self,
        start_delimiter: &str,
        end_delimiter: &str,
        token_type: &'static str,
        token_sub_type: Option<&'static str>,
        allow_nesting: bool,
        raw_mode: bool,
        include_delimiters: bool,
    ) {
        let scanner = ScannerType::Block(BlockScanner::new(
            start_delimiter,
            end_delimiter,
            token_type,
            token_sub_type,
            allow_nesting,
            raw_mode,
            include_delimiters,
        ));
        self.scanners.push(scanner);
    }

    /// Adds an End-of-Line scanner to the tokenizer.
    /// This scanner matches content that starts with a specific delimiter and continues until a newline.
    ///
    /// # Arguments
    /// * `delimiter` - The delimiter that marks the beginning of the line-based structure
    /// * `token_type` - The type of token to create for matched content
    /// * `token_sub_type` - Optional subtype for more specific token categorization
    /// * `include_delimiter` - Whether to include the delimiter in the token value
    pub fn add_eol_scanner(
        &mut self,
        delimiter: &str,
        token_type: &'static str,
        token_sub_type: Option<&'static str>,
        include_delimiter: bool,
    ) {
        let scanner = ScannerType::Eol(EolScanner::new(
            delimiter,
            token_type,
            token_sub_type,
            include_delimiter,
        ));
        self.scanners.push(scanner);
    }

    /// Register a [`ContextualScanner`] — a scanner that receives mutable access to
    /// [`ScanContext`] on every scan attempt, enabling lexer-mode switching.
    ///
    /// Contextual scanners are **invisible** to the standard [`tokenize`](Self::tokenize)
    /// method; use [`tokenize_contextual`](Self::tokenize_contextual) instead.
    pub fn add_contextual_scanner(&mut self, scanner: Box<dyn ContextualScanner>) {
        self.scanners.push(ScannerType::Contextual(scanner));
    }

    /// Register a closure as a [`ContextualScanner`].
    ///
    /// The closure receives the remaining input slice and a `&mut ScanContext`.
    /// Write to `ctx.mode` inside the closure to switch the tokenizer's lexer mode.
    pub fn add_contextual_closure(
        &mut self,
        cb: impl for<'i> Fn(&'i str, &mut ScanContext) -> Result<Option<Token<'i>>, TokenizationError>
            + Send
            + Sync
            + 'static,
    ) {
        self.scanners
            .push(ScannerType::Contextual(Box::new(ContextualClosureScanner::new(cb))));
    }

    /// Register a [`KeywordScanner`] for a list of reserved words.
    ///
    /// Automatically applies word-boundary checking so `if` never matches inside
    /// `ifdef`.  Keywords are tried longest-first.
    ///
    /// ```rust,ignore
    /// tokenizer.add_keyword_scanner("Keyword", &["if", "else", "while", "return"]);
    /// ```
    pub fn add_keyword_scanner(&mut self, token_type: &'static str, keywords: &[&str]) {
        self.scanners
            .push(ScannerType::Keyword(KeywordScanner::new(token_type, keywords)));
    }

    /// Register a [`KeywordScanner`] where each keyword gets a distinct `token_sub_type`.
    ///
    /// ```rust,ignore
    /// tokenizer.add_keyword_scanner_with_subtypes("Keyword", &[
    ///     ("if",     "If"),
    ///     ("else",   "Else"),
    ///     ("return", "Return"),
    /// ]);
    /// ```
    pub fn add_keyword_scanner_with_subtypes(
        &mut self,
        token_type: &'static str,
        keywords: &[(&str, &'static str)],
    ) {
        self.scanners
            .push(ScannerType::Keyword(KeywordScanner::with_subtypes(token_type, keywords)));
    }

    /// Register a [`CharClassScanner`] using a lead character-class spec and an
    /// optional continuation class spec.
    ///
    /// Spec syntax: `"a-zA-Z_"` (range) + `"_!?"` (literals) combined freely.
    ///
    /// ```rust,ignore
    /// // Standard identifier: [a-zA-Z_][a-zA-Z0-9_]*
    /// tokenizer.add_char_class_scanner("a-zA-Z_", Some("a-zA-Z0-9_"), "Identifier", None);
    ///
    /// // Single-char operator symbols: [+\-*/<>=!]
    /// tokenizer.add_char_class_scanner("+-*/<=!>", None, "Operator", None);
    /// ```
    pub fn add_char_class_scanner(
        &mut self,
        lead_spec: &str,
        continuation_spec: Option<&str>,
        token_type: &'static str,
        token_sub_type: Option<&'static str>,
    ) {
        self.scanners.push(ScannerType::CharClass(CharClassScanner::new(
            lead_spec,
            continuation_spec,
            token_type,
            token_sub_type,
        )));
    }

    /// Register a [`NumberLiteralScanner`] with full defaults (hex, binary, octal,
    /// float, scientific notation, underscore separators all enabled).
    ///
    /// ```rust,ignore
    /// tokenizer.add_number_literal_scanner("Number", None);
    /// ```
    ///
    /// For finer control, build the scanner directly and register with
    /// [`add_scanner`](Self::add_scanner):
    ///
    /// ```rust,ignore
    /// use rb_tokenizer::scanners::NumberLiteralScanner;
    /// tokenizer.add_scanner(Box::new(
    ///     NumberLiteralScanner::minimal("Number", None)
    ///         .allow_float(true)
    ///         .allow_scientific(true),
    /// ));
    /// ```
    pub fn add_number_literal_scanner(
        &mut self,
        token_type: &'static str,
        token_sub_type: Option<&'static str>,
    ) {
        self.scanners.push(ScannerType::NumberLiteral(
            NumberLiteralScanner::new(token_type, token_sub_type),
        ));
    }

    /// Register an [`OperatorScanner`] where all operators share one `token_type`.
    ///
    /// Operators are matched **longest-first** with **no word-boundary check**, making
    /// this the right choice for symbolic operators like `++`, `+=`, `->`, `<<=`.
    ///
    /// ```rust,ignore
    /// tokenizer.add_operator_scanner("Op", &["+=", "-=", "++", "--", "+", "-", "="]);
    /// ```
    pub fn add_operator_scanner(&mut self, token_type: &'static str, operators: &[&str]) {
        self.scanners
            .push(ScannerType::Operator(OperatorScanner::new(token_type, operators)));
    }

    /// Register an [`OperatorScanner`] where each operator gets a distinct `token_sub_type`.
    ///
    /// ```rust,ignore
    /// tokenizer.add_operator_scanner_with_subtypes("Op", &[
    ///     ("<<=", "ShlAssign"),
    ///     ("<<",  "Shl"),
    ///     ("<=",  "Le"),
    ///     ("<",   "Lt"),
    /// ]);
    /// ```
    pub fn add_operator_scanner_with_subtypes(
        &mut self,
        token_type: &'static str,
        operators: &[(&str, &'static str)],
    ) {
        self.scanners
            .push(ScannerType::Operator(OperatorScanner::with_subtypes(token_type, operators)));
    }

    /// Register a [`WhitespaceScanner`] for whitespace handling.
    ///
    /// Use the named constructors on [`WhitespaceScanner`] to choose a mode:
    ///
    /// ```rust,ignore
    /// use rb_tokenizer::WhitespaceScanner;
    ///
    /// // Uniform — all whitespace as one token (C, Java, JSON)
    /// tokenizer.add_whitespace_scanner(WhitespaceScanner::uniform("Whitespace"));
    ///
    /// // Split — separate Newline tokens (Go, JavaScript, Ruby, Kotlin)
    /// tokenizer.add_whitespace_scanner(WhitespaceScanner::split("Whitespace", "Newline"));
    ///
    /// // With continuation — split + backslash-newline (C preprocessor, Python, Bash)
    /// tokenizer.add_whitespace_scanner(WhitespaceScanner::with_continuation(
    ///     "Whitespace", "Newline", "LineContinuation",
    /// ));
    /// ```
    pub fn add_whitespace_scanner(&mut self, scanner: WhitespaceScanner) {
        self.scanners.push(ScannerType::Whitespace(scanner));
    }

    /// Returns `true` if any contextual scanner has been registered via
    /// [`add_contextual_scanner`](Self::add_contextual_scanner) or
    /// [`add_contextual_closure`](Self::add_contextual_closure).
    ///
    /// Use this to select between [`tokenize`](Self::tokenize) and
    /// [`tokenize_contextual`](Self::tokenize_contextual) at runtime.
    pub fn has_contextual_scanners(&self) -> bool {
        self.scanners.iter().any(|s| matches!(s, ScannerType::Contextual(_)))
    }

    // Enhanced tokenize method with improved whitespace handling
    pub fn tokenize<'i>(&self, input: &'i str) -> Result<Vec<Token<'i>>, Vec<TokenizationError>> {
        // Guard: contextual scanners are invisible to this path. Catch the mistake early.
        #[cfg(debug_assertions)]
        if self.has_contextual_scanners() {
            panic!(
                "Tokenizer::tokenize() called but contextual scanners are registered. \
                 Use tokenize_contextual() instead, or check has_contextual_scanners() before calling."
            );
        }

        let mut tokens = Vec::with_capacity(input.len() / 6);
        let mut errors = Vec::new();

        let mut current_line: usize = 1;
        let mut current_column: usize = 1;
        let mut pos: usize = 0;

        while pos < input.len() {
            let current_input = &input[pos..];
            let mut matched = false;

            for scanner in &self.scanners {
                match scanner.scan_with_context(current_input) {
                    Ok(Some(scan_match)) => {
                        let token_len = scan_match.consumed_len;
                        let partial = scan_match.token;

                        let start_byte = pos;
                        let start_pos = SourcePosition {
                            byte_offset: start_byte,
                            line: current_line - 1,
                            column: current_column - 1,
                        };

                        Self::advance_pos(
                            input[pos..pos + token_len].as_bytes(),
                            &mut current_line,
                            &mut current_column,
                        );
                        pos += token_len;

                        let span = if self.config.track_token_positions {
                            SourceSpan {
                                source_id: self.source_id,
                                start: start_pos,
                                end: SourcePosition {
                                    byte_offset: pos,
                                    line: current_line - 1,
                                    column: current_column - 1,
                                },
                            }
                        } else {
                            SourceSpan::UNKNOWN
                        };

                        tokens.push(Token {
                            token_type: partial.token_type,
                            token_sub_type: partial.token_sub_type,
                            value: partial.value,
                            span,
                        });

                        matched = true;
                        break;
                    }
                    Ok(None) => {}
                    Err(e) => {
                        let err_span = SourceSpan {
                            source_id: self.source_id,
                            start: SourcePosition { byte_offset: pos, line: current_line - 1, column: current_column - 1 },
                            end: SourcePosition { byte_offset: pos, line: current_line - 1, column: current_column - 1 },
                        };
                        errors.push(e.at(err_span));

                        if errors.len() >= self.config.error_tolerance_limit {
                            *self.last_errors.borrow_mut() = Some(errors.clone());
                            return Err(errors);
                        }

                        // If we encounter an error but want to continue, skip this character.
                        if self.config.continue_on_error {
                            // Skip the current character without decoding it as a char.
                            let b0 = current_input.as_bytes()[0];
                            pos += if b0 < 0x80 { 1 } else if b0 < 0xE0 { 2 } else if b0 < 0xF0 { 3 } else { 4 };
                            current_column += 1;
                            matched = true; // Mark as matched so we don't double-count this error
                            break;
                        } else {
                            *self.last_errors.borrow_mut() = Some(errors.clone());
                            return Err(errors);
                        }
                    }
                }
            }

            if !matched {
                let b0 = current_input.as_bytes()[0]; // safe: pos < input.len()
                // Fast ASCII whitespace check; only decode the char for non-ASCII code points.
                let (is_ws, decoded_char) = if b0.is_ascii() {
                    (matches!(b0, b' ' | b'\t' | b'\n' | b'\r' | 0x0B | 0x0C), None)
                } else {
                    let ch = current_input.chars().next().unwrap();
                    (ch.is_whitespace(), Some(ch))
                };
                if is_ws {
                    if self.config.tokenize_whitespace {
                        let ws_start_byte = pos;
                        let ws_start_line = current_line;
                        let ws_start_col = current_column;
                        let mut has_newline = false;

                        while pos < input.len() {
                            match input.as_bytes()[pos] {
                                b'\n' => { has_newline = true; current_line += 1; current_column = 1; pos += 1; }
                                b' ' | b'\t' | b'\r' | 0x0B | 0x0C => { current_column += 1; pos += 1; }
                                0x80..=0xFF => {
                                    let ch = input[pos..].chars().next().unwrap();
                                    if !ch.is_whitespace() { break; }
                                    if ch == '\n' { has_newline = true; current_line += 1; current_column = 1; }
                                    else { current_column += 1; }
                                    pos += ch.len_utf8();
                                }
                                _ => break,
                            }
                        }

                        let ws_span = if self.config.track_token_positions {
                            SourceSpan {
                                source_id: self.source_id,
                                start: SourcePosition {
                                    byte_offset: ws_start_byte,
                                    line: ws_start_line - 1,
                                    column: ws_start_col - 1,
                                },
                                end: SourcePosition {
                                    byte_offset: pos,
                                    line: current_line - 1,
                                    column: current_column - 1,
                                },
                            }
                        } else {
                            SourceSpan::UNKNOWN
                        };

                        tokens.push(Token {
                            token_type: "Whitespace",
                            token_sub_type: if has_newline { Some("Newline") } else { None },
                            value: Cow::Borrowed(&input[ws_start_byte..pos]),
                            span: ws_span,
                        });
                    } else {
                        // Skip whitespace byte-by-byte; fallback to char decode for non-ASCII.
                        while pos < input.len() {
                            match input.as_bytes()[pos] {
                                b'\n' => { current_line += 1; current_column = 1; pos += 1; }
                                b' ' | b'\t' | b'\r' | 0x0B | 0x0C => { current_column += 1; pos += 1; }
                                0x80..=0xFF => {
                                    let ch = input[pos..].chars().next().unwrap();
                                    if !ch.is_whitespace() { break; }
                                    if ch == '\n' { current_line += 1; current_column = 1; }
                                    else { current_column += 1; }
                                    pos += ch.len_utf8();
                                }
                                _ => break,
                            }
                        }
                    }
                } else {
                    let next_char = decoded_char.unwrap_or(b0 as char);
                    let err_span = SourceSpan {
                        source_id: self.source_id,
                        start: SourcePosition { byte_offset: pos, line: current_line - 1, column: current_column - 1 },
                        end: SourcePosition { byte_offset: pos, line: current_line - 1, column: current_column - 1 },
                    };
                    let error = TokenizationError::UnrecognizedToken(
                        format!("Unrecognized token at line {}, column {}: '{}'",
                            current_line, current_column, next_char)
                    ).at(err_span);
                    errors.push(error);

                    if self.config.continue_on_error {
                        pos += if b0 < 0x80 { 1 } else if b0 < 0xE0 { 2 } else if b0 < 0xF0 { 3 } else { 4 };
                        current_column += 1;
                    } else {
                        break;
                    }
                }
            }
        }

        if errors.is_empty() {
            *self.last_errors.borrow_mut() = None;
            Ok(tokens)
        } else if self.config.continue_on_error {
            *self.last_errors.borrow_mut() = Some(errors.clone());
            Ok(tokens)
        } else {
            *self.last_errors.borrow_mut() = Some(errors.clone());
            Err(errors)
        }
    }

    /// Mode-aware tokenization.  Like [`tokenize`](Self::tokenize) but also drives
    /// [`Contextual`](crate::scanners::ScannerType::Contextual) scanners registered
    /// with [`add_contextual_scanner`](Self::add_contextual_scanner) or
    /// [`add_contextual_closure`](Self::add_contextual_closure).
    ///
    /// A [`ScanContext`] is maintained internally and updated after every emitted token:
    /// - `ctx.line` / `ctx.column` — byte-exact position at the start of the next attempt
    /// - `ctx.prev_token_kind` — `token_type` of the most recently emitted token
    /// - `ctx.mode` — whatever value the last contextual scanner wrote; preserved across calls
    ///
    /// Non-contextual scanners (regex, symbol, block, etc.) run unaffected — they ignore
    /// the context and are tried in registration order alongside contextual ones.
    pub fn tokenize_contextual<'i>(
        &self,
        input: &'i str,
    ) -> Result<Vec<Token<'i>>, Vec<TokenizationError>> {
        let mut tokens = Vec::with_capacity(input.len() / 6);
        let mut errors = Vec::new();
        let mut ctx = ScanContext::new();

        let mut current_line = 1usize;
        let mut current_column = 1usize;
        let mut pos: usize = 0;

        while pos < input.len() {
            let current_input = &input[pos..];
            let mut matched = false;

            ctx.line = current_line;
            ctx.column = current_column;

            for scanner in &self.scanners {
                match scanner.scan_contextually(current_input, &mut ctx) {
                    Ok(Some(scan_match)) => {
                        let token_len = scan_match.consumed_len;
                        let partial = scan_match.token;

                        let start_byte = pos;
                        let start_pos = SourcePosition {
                            byte_offset: start_byte,
                            line: current_line - 1,
                            column: current_column - 1,
                        };

                        Self::advance_pos(
                            input[pos..pos + token_len].as_bytes(),
                            &mut current_line,
                            &mut current_column,
                        );
                        pos += token_len;

                        let span = if self.config.track_token_positions {
                            SourceSpan {
                                source_id: self.source_id,
                                start: start_pos,
                                end: SourcePosition {
                                    byte_offset: pos,
                                    line: current_line - 1,
                                    column: current_column - 1,
                                },
                            }
                        } else {
                            SourceSpan::UNKNOWN
                        };

                        let token = Token {
                            token_type: partial.token_type,
                            token_sub_type: partial.token_sub_type,
                            value: partial.value,
                            span,
                        };

                        ctx.prev_token_kind = Some(token.token_type);
                        tokens.push(token);

                        matched = true;
                        break;
                    }
                    Ok(None) => {}
                    Err(e) => {
                        let err_span = SourceSpan {
                            source_id: self.source_id,
                            start: SourcePosition { byte_offset: pos, line: current_line - 1, column: current_column - 1 },
                            end: SourcePosition { byte_offset: pos, line: current_line - 1, column: current_column - 1 },
                        };
                        errors.push(e.at(err_span));
                        if errors.len() >= self.config.error_tolerance_limit {
                            *self.last_errors.borrow_mut() = Some(errors.clone());
                            return Err(errors);
                        }
                        if self.config.continue_on_error {
                            let b0 = current_input.as_bytes()[0];
                            pos += if b0 < 0x80 { 1 } else if b0 < 0xE0 { 2 } else if b0 < 0xF0 { 3 } else { 4 };
                            current_column += 1;
                            matched = true;
                            break;
                        } else {
                            *self.last_errors.borrow_mut() = Some(errors.clone());
                            return Err(errors);
                        }
                    }
                }
            }

            if !matched {
                let b0 = current_input.as_bytes()[0]; // safe: pos < input.len()
                let (is_ws, decoded_char) = if b0.is_ascii() {
                    (matches!(b0, b' ' | b'\t' | b'\n' | b'\r' | 0x0B | 0x0C), None)
                } else {
                    let ch = current_input.chars().next().unwrap();
                    (ch.is_whitespace(), Some(ch))
                };
                if is_ws {
                    if self.config.tokenize_whitespace {
                        let ws_start_byte = pos;
                        let start_line = current_line;
                        let start_column = current_column;
                        let mut has_newline = false;

                        while pos < input.len() {
                            match input.as_bytes()[pos] {
                                b'\n' => { has_newline = true; current_line += 1; current_column = 1; pos += 1; }
                                b' ' | b'\t' | b'\r' | 0x0B | 0x0C => { current_column += 1; pos += 1; }
                                0x80..=0xFF => {
                                    let ch = input[pos..].chars().next().unwrap();
                                    if !ch.is_whitespace() { break; }
                                    if ch == '\n' { has_newline = true; current_line += 1; current_column = 1; }
                                    else { current_column += 1; }
                                    pos += ch.len_utf8();
                                }
                                _ => break,
                            }
                        }

                        let ws_span = if self.config.track_token_positions {
                            SourceSpan {
                                source_id: self.source_id,
                                start: SourcePosition {
                                    byte_offset: ws_start_byte,
                                    line: start_line - 1,
                                    column: start_column - 1,
                                },
                                end: SourcePosition {
                                    byte_offset: pos,
                                    line: current_line - 1,
                                    column: current_column - 1,
                                },
                            }
                        } else {
                            SourceSpan::UNKNOWN
                        };

                        let ws_token = Token {
                            token_type: "Whitespace",
                            token_sub_type: if has_newline { Some("Newline") } else { None },
                            value: Cow::Borrowed(&input[ws_start_byte..pos]),
                            span: ws_span,
                        };
                        ctx.prev_token_kind = Some(ws_token.token_type);
                        tokens.push(ws_token);
                    } else {
                        // Skip whitespace byte-by-byte; fallback to char decode for non-ASCII.
                        while pos < input.len() {
                            match input.as_bytes()[pos] {
                                b'\n' => { current_line += 1; current_column = 1; pos += 1; }
                                b' ' | b'\t' | b'\r' | 0x0B | 0x0C => { current_column += 1; pos += 1; }
                                0x80..=0xFF => {
                                    let ch = input[pos..].chars().next().unwrap();
                                    if !ch.is_whitespace() { break; }
                                    if ch == '\n' { current_line += 1; current_column = 1; }
                                    else { current_column += 1; }
                                    pos += ch.len_utf8();
                                }
                                _ => break,
                            }
                        }
                    }
                } else {
                    let next_char = decoded_char.unwrap_or(b0 as char);
                    let err_span = SourceSpan {
                        source_id: self.source_id,
                        start: SourcePosition { byte_offset: pos, line: current_line - 1, column: current_column - 1 },
                        end: SourcePosition { byte_offset: pos, line: current_line - 1, column: current_column - 1 },
                    };
                    let error = TokenizationError::UnrecognizedToken(
                        format!("Unrecognized token at line {}, column {}: '{}'",
                            current_line, current_column, next_char)
                    ).at(err_span);
                    errors.push(error);
                    if self.config.continue_on_error {
                        pos += if b0 < 0x80 { 1 } else if b0 < 0xE0 { 2 } else if b0 < 0xF0 { 3 } else { 4 };
                        current_column += 1;
                    } else {
                        break;
                    }
                }
            }
        }

        if errors.is_empty() {
            *self.last_errors.borrow_mut() = None;
            Ok(tokens)
        } else if self.config.continue_on_error {
            *self.last_errors.borrow_mut() = Some(errors.clone());
            Ok(tokens)
        } else {
            *self.last_errors.borrow_mut() = Some(errors.clone());
            Err(errors)
        }
    }

    /// Sets whether the tokenizer should continue on errors
    pub fn set_continue_on_error(&mut self, value: bool) -> &mut Self {
        self.config.continue_on_error = value;
        self
    }

    /// Sets whether the tokenizer should tokenize whitespace
    pub fn set_tokenize_whitespace(&mut self, value: bool) -> &mut Self {
        self.config.tokenize_whitespace = value;
        self
    }

    /// Sets the maximum number of errors before tokenization fails
    pub fn set_error_tolerance_limit(&mut self, value: usize) -> &mut Self {
        self.config.error_tolerance_limit = value;
        self
    }

    /// Sets whether the tokenizer should track token positions
    pub fn set_track_token_positions(&mut self, value: bool) -> &mut Self {
        self.config.track_token_positions = value;
        self
    }

    /// Updates the tokenizer configuration with the provided values
    pub fn with_options(&mut self,
        continue_on_error: Option<bool>,
        tokenize_whitespace: Option<bool>,
        error_tolerance_limit: Option<usize>,
        track_token_positions: Option<bool>
    ) -> &mut Self {
        if let Some(val) = continue_on_error {
            self.config.continue_on_error = val;
        }

        if let Some(val) = tokenize_whitespace {
            self.config.tokenize_whitespace = val;
        }

        if let Some(val) = error_tolerance_limit {
            self.config.error_tolerance_limit = val;
        }

        if let Some(val) = track_token_positions {
            self.config.track_token_positions = val;
        }

        self
    }
}
