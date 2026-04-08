use crate::scanners::char_class_scanner::CharClassScanner;
use crate::scanners::contextual_scanner::{ContextualClosureScanner, ContextualScanner};
use crate::scanners::keyword_scanner::KeywordScanner;
use crate::scanners::number_literal_scanner::NumberLiteralScanner;
use crate::scanners::scan_context::ScanContext;
use crate::scanners::{self, BlockScanner, EolScanner, RegexScanner, Scanner, ScannerType, SymbolScanner};
use crate::tokens::{Token, TokenizationError};
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
}

impl Default for Tokenizer {
    fn default() -> Self {
        Self::new()
    }
}

impl Tokenizer {
    fn advance_cursor(
        chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
        consumed_len: usize,
        current_line: &mut usize,
        current_column: &mut usize,
    ) {
        let mut consumed_bytes = 0;

        while consumed_bytes < consumed_len {
            if let Some((_, ch)) = chars.next() {
                consumed_bytes += ch.len_utf8();
                if ch == '\n' {
                    *current_line += 1;
                    *current_column = 1;
                } else {
                    *current_column += 1;
                }
            } else {
                break;
            }
        }
    }

    pub fn new() -> Self {
        Tokenizer {
            scanners: Vec::new(),
            config: TokenizerConfig::default(),
            last_errors: RefCell::new(None),
        }
    }

    pub fn with_config(config: TokenizerConfig) -> Self {
        Tokenizer {
            scanners: Vec::new(),
            config,
            last_errors: RefCell::new(None),
        }
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
        cb: impl Fn(&str, &mut ScanContext) -> Result<Option<Token>, TokenizationError>
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

    // Enhanced tokenize method with improved whitespace handling
    pub fn tokenize(&self, input: &str) -> Result<Vec<Token>, Vec<TokenizationError>> {
        let mut tokens = Vec::new();
        let mut errors = Vec::new();

        let mut current_line = 1;
        let mut current_column = 1;
        let mut chars = input.char_indices().peekable();

        while let Some((start, next_char)) = chars.peek().copied() {
            let mut matched = false;
            let current_input = &input[start..];

            // Try to match complex scanners first (like strings which can contain whitespace)
            for scanner in &self.scanners {
                match scanner.scan_with_context(current_input) {
                    Ok(Some(scan_match)) => {
                        let token_len = scan_match.consumed_len;
                        let token = scan_match.token;

                        // Track position if configured
                        let token_with_position = if self.config.track_token_positions {
                            Token {
                                line: current_line,
                                column: current_column,
                                ..token
                            }
                        } else {
                            token
                        };

                        tokens.push(token_with_position);

                        // Advance the iterator and update positions
                        Self::advance_cursor(&mut chars, token_len, &mut current_line, &mut current_column);
                        matched = true;
                        break;
                    }
                    Ok(None) => {}
                    Err(e) => {
                        // Preserve the original error and add position information
                        errors.push(e);

                        if errors.len() >= self.config.error_tolerance_limit {
                            *self.last_errors.borrow_mut() = Some(errors.clone());
                            return Err(errors);
                        }

                        // If we encounter an error but want to continue, we need to skip this character
                        if self.config.continue_on_error {
                            chars.next();
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
                if next_char.is_whitespace() {
                    if self.config.tokenize_whitespace {
                        let start_line = current_line;
                        let start_column = current_column;
                        let mut whitespace = String::new();
                        let mut has_newline = false;

                        // Consume whitespace characters one by one
                        while let Some((_, ch)) = chars.peek() {
                            if !ch.is_whitespace() {
                                break;
                            }

                            // Clone the character to avoid borrowing issues
                            let current_char = *ch;
                            whitespace.push(current_char);
                            has_newline |= current_char == '\n';

                            // Advance the iterator
                            chars.next();

                            if current_char == '\n' {
                                current_line += 1;
                                current_column = 1;
                            } else {
                                current_column += 1;
                            }
                        }

                        // Create the whitespace token
                        tokens.push(Token {
                            token_type: "Whitespace",
                            token_sub_type: if has_newline { Some("Newline") } else { None },
                            value: whitespace,
                            line: start_line,
                            column: start_column,
                        });
                    } else {
                        // Skip whitespace when not tokenizing it
                        while let Some((_, ch)) = chars.peek() {
                            if !ch.is_whitespace() {
                                break;
                            }

                            let current_char = *ch;
                            chars.next();

                            if current_char == '\n' {
                                current_line += 1;
                                current_column = 1;
                            } else {
                                current_column += 1;
                            }
                        }
                    }
                } else {
                    let error = TokenizationError::UnrecognizedToken(
                        format!("Unrecognized token at line {}, column {}: '{}'",
                            current_line, current_column, next_char)
                    );
                    errors.push(error);

                    if self.config.continue_on_error {
                        chars.next();
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
    pub fn tokenize_contextual(
        &self,
        input: &str,
    ) -> Result<Vec<Token>, Vec<TokenizationError>> {
        let mut tokens = Vec::new();
        let mut errors = Vec::new();
        let mut ctx = ScanContext::new();

        let mut current_line = 1usize;
        let mut current_column = 1usize;
        let mut chars = input.char_indices().peekable();

        while let Some((start, next_char)) = chars.peek().copied() {
            let mut matched = false;
            let current_input = &input[start..];

            ctx.line = current_line;
            ctx.column = current_column;

            for scanner in &self.scanners {
                match scanner.scan_contextually(current_input, &mut ctx) {
                    Ok(Some(scan_match)) => {
                        let token_len = scan_match.consumed_len;
                        let token = scan_match.token;

                        let token_with_position = if self.config.track_token_positions {
                            Token {
                                line: current_line,
                                column: current_column,
                                ..token
                            }
                        } else {
                            token
                        };

                        ctx.prev_token_kind = Some(token_with_position.token_type);
                        tokens.push(token_with_position);

                        Self::advance_cursor(&mut chars, token_len, &mut current_line, &mut current_column);
                        matched = true;
                        break;
                    }
                    Ok(None) => {}
                    Err(e) => {
                        errors.push(e);
                        if errors.len() >= self.config.error_tolerance_limit {
                            *self.last_errors.borrow_mut() = Some(errors.clone());
                            return Err(errors);
                        }
                        if self.config.continue_on_error {
                            chars.next();
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
                if next_char.is_whitespace() {
                    if self.config.tokenize_whitespace {
                        let start_line = current_line;
                        let start_column = current_column;
                        let mut whitespace = String::new();
                        let mut has_newline = false;

                        while let Some((_, ch)) = chars.peek() {
                            if !ch.is_whitespace() { break; }
                            let current_char = *ch;
                            whitespace.push(current_char);
                            has_newline |= current_char == '\n';
                            chars.next();
                            if current_char == '\n' {
                                current_line += 1;
                                current_column = 1;
                            } else {
                                current_column += 1;
                            }
                        }

                        let ws_token = Token {
                            token_type: "Whitespace",
                            token_sub_type: if has_newline { Some("Newline") } else { None },
                            value: whitespace,
                            line: start_line,
                            column: start_column,
                        };
                        ctx.prev_token_kind = Some(ws_token.token_type);
                        tokens.push(ws_token);
                    } else {
                        while let Some((_, ch)) = chars.peek() {
                            if !ch.is_whitespace() { break; }
                            let current_char = *ch;
                            chars.next();
                            if current_char == '\n' {
                                current_line += 1;
                                current_column = 1;
                            } else {
                                current_column += 1;
                            }
                        }
                    }
                } else {
                    let error = TokenizationError::UnrecognizedToken(
                        format!("Unrecognized token at line {}, column {}: '{}'",
                            current_line, current_column, next_char)
                    );
                    errors.push(error);
                    if self.config.continue_on_error {
                        chars.next();
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
