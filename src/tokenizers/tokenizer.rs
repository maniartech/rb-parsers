use crate::scanners::{self, RegexScanner, Scanner, ScannerType, SymbolScanner};
use crate::tokens::{Token, TokenizationError};

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
}

impl Tokenizer {
    pub fn new() -> Self {
        Tokenizer {
            scanners: Vec::new(),
            config: TokenizerConfig::default(),
        }
    }

    pub fn with_config(config: TokenizerConfig) -> Self {
        Tokenizer {
            scanners: Vec::new(),
            config,
        }
    }

    pub fn config(&self) -> &TokenizerConfig {
        &self.config
    }

    pub fn config_mut(&mut self) -> &mut TokenizerConfig {
        &mut self.config
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
        token_type: &str,
        sub_token_type: Option<&str>,
    ) {
        let scanner = ScannerType::Regex(RegexScanner::new(pattern, token_type, sub_token_type));
        self.scanners.push(scanner);
    }

    pub fn add_symbol_scanner(&mut self, symbol: &str, token_type: &str, default_scanner: Option<&str>) {
        let scanner = ScannerType::Symbol(SymbolScanner::new(symbol, token_type, default_scanner));
        self.scanners.push(scanner);
    }

    pub fn add_closure_scanner(
        &mut self,
        cb: Box<dyn Fn(&str) -> Result<Option<Token>, TokenizationError>>,
    ) {
        let scanner = ScannerType::Closure(scanners::ClosureScanner::new(cb));
        self.scanners.push(scanner);
    }

    pub fn add_callback_scanner(&mut self, cb: Box<dyn scanners::CallbackScanner>) {
        let scanner = ScannerType::Callback(cb);
        self.scanners.push(scanner);
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
                match scanner.scan(current_input) {
                    Ok(Some(token)) => {
                        let token_len = token.value.len();
                        
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
                        for _ in 0..token_len {
                            if let Some((_, char)) = chars.next() {
                                if char == '\n' {
                                    current_line += 1;
                                    current_column = 1;
                                } else {
                                    current_column += 1;
                                }
                            }
                        }
                        matched = true;
                        break;
                    }
                    Ok(None) => {}
                    Err(e) => {
                        let error_message = format!(
                            "Error while scanning input: {:?} at line {} column {}",
                            e, current_line, current_column
                        );
                        eprintln!("{}", error_message);
                        
                        let error = TokenizationError::UnrecognizedToken(
                            format!("Unrecognized token at line {}, column {}: '{}'", 
                                current_line, current_column, next_char)
                        );
                        
                        errors.push(error);
                        
                        if errors.len() >= self.config.error_tolerance_limit {
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
                            token_type: String::from("Whitespace"),
                            token_sub_type: if has_newline { Some(String::from("Newline")) } else { None },
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
            Ok(tokens)
        } else {
            Err(errors)
        }
    }
}
