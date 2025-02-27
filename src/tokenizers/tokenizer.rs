use crate::rules::{self, RegexRule, Rule, RuleType, SymbolRule};
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
    rules: Vec<RuleType>,
    config: TokenizerConfig,
}

impl Tokenizer {
    pub fn new() -> Self {
        Tokenizer {
            rules: Vec::new(),
            config: TokenizerConfig::default(),
        }
    }

    pub fn with_config(config: TokenizerConfig) -> Self {
        Tokenizer {
            rules: Vec::new(),
            config,
        }
    }

    pub fn config(&self) -> &TokenizerConfig {
        &self.config
    }

    pub fn config_mut(&mut self) -> &mut TokenizerConfig {
        &mut self.config
    }

    pub fn add_rule(&mut self, rule: Box<dyn rules::Rule>) {
        self.rules.push(RuleType::Rule(rule));
    }

    pub fn add_rule_with_priority(&mut self, rule: Box<dyn rules::Rule>, priority: usize) {
        // Insert rule at the specified priority (lower index = higher priority)
        if priority >= self.rules.len() {
            self.rules.push(RuleType::Rule(rule));
        } else {
            self.rules.insert(priority, RuleType::Rule(rule));
        }
    }

    pub fn add_regex_rule(
        &mut self,
        pattern: &str,
        token_type: &str,
        sub_token_type: Option<&str>,
    ) {
        let rule = RuleType::Regex(RegexRule::new(pattern, token_type, sub_token_type));
        self.rules.push(rule);
    }

    pub fn add_symbol_rule(&mut self, symbol: &str, token_type: &str, default_rule: Option<&str>) {
        let rule = RuleType::Symbol(SymbolRule::new(symbol, token_type, default_rule));
        self.rules.push(rule);
    }

    pub fn add_closure_rule(
        &mut self,
        cb: Box<dyn Fn(&str) -> Result<Option<Token>, TokenizationError>>,
    ) {
        let rule = RuleType::Closure(rules::ClosureRule::new(cb));
        self.rules.push(rule);
    }

    pub fn add_callback_rule(&mut self, cb: Box<dyn rules::CallbackRule>) {
        let rule = RuleType::Callback(cb);
        self.rules.push(rule);
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
            
            // Try to match complex rules first (like strings which can contain whitespace)
            for rule in &self.rules {
                match rule.process(current_input) {
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
                            "Error while processing input: {:?} at line {} column {}",
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
