use crate::rules::{self, CallbackRule, RegexRule, Rule, RuleType, SymbolRule};
use crate::tokens::{Token, TokenizationError};

#[derive(Debug, Clone)]
pub struct TokenizerConfig {
    pub tokenize_whitespace: bool,
    pub continue_on_error: bool,
    pub error_tolerance_limit: usize,
    pub preserve_whitespace_in_tokens: bool, // Controls whitespace preservation in tokens like strings
    pub track_token_positions: bool,        // Controls whether line/column tracking is performed
}

impl Default for TokenizerConfig {
    fn default() -> Self {
        Self {
            tokenize_whitespace: false,
            continue_on_error: false,
            error_tolerance_limit: 10,
            preserve_whitespace_in_tokens: true, // Default to preserving whitespace in tokens like strings
            track_token_positions: true,        // Default to tracking positions
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
        let mut current_column = 1; // Start column counting from 1
        let mut chars = input.char_indices().peekable();

        while let Some((start, next_char)) = chars.peek().copied() {
            let mut matched = false;
            let current_input = &input[start..];
            
            // Try to match complex rules first (like strings which can contain whitespace)
            // This ensures proper handling of tokens like strings that contain whitespace
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
                        
                        // Advance the iterator by token_len characters and update positions
                        for _ in 0..token_len {
                            if let Some((_, char)) = chars.next() {
                                // Update position counters
                                if char == '\n' {
                                    current_line += 1;
                                    current_column = 1;
                                } else {
                                    current_column += 1;
                                }
                            }
                        }
                        matched = true;
                        break; // Break to process the next segment of input
                    }
                    Ok(None) => {} // No match, continue to next rule
                    Err(e) => {
                        let error_message = format!(
                            "Error while processing input: {:?} at line {} column {}",
                            e, current_line, current_column
                        );
                        eprintln!("{}", error_message);
                        
                        let error = if let TokenizationError::UnrecognizedToken(_) = e {
                            TokenizationError::UnrecognizedToken(
                                format!("Unrecognized token at line {}, column {}: '{}'", 
                                    current_line, current_column, next_char)
                            )
                        } else {
                            e.clone()
                        };
                        
                        errors.push(error);
                        
                        if errors.len() >= self.config.error_tolerance_limit {
                            return Err(errors);
                        }
                    }
                }
            }

            // If no rule matched, handle whitespace or report an error
            if !matched {
                if next_char.is_whitespace() {
                    if self.config.tokenize_whitespace {
                        // Create a whitespace token
                        let whitespace_value = next_char.to_string();
                        tokens.push(Token {
                            token_type: String::from("Whitespace"),
                            token_sub_type: if next_char == '\n' { Some(String::from("Newline")) } else { None },
                            value: whitespace_value,
                            line: current_line,
                            column: current_column,
                        });
                    }
                    
                    // Update line and column counters
                    if next_char == '\n' {
                        current_line += 1;
                        current_column = 1; // Reset column at new line
                    } else {
                        current_column += 1; // Increment column for spaces
                    }
                    chars.next(); // Consume the whitespace character
                } else {
                    // If no rules matched and it's not whitespace, record the error
                    let error = TokenizationError::UnrecognizedToken(
                        format!("Unrecognized token at line {}, column {}: '{}'", 
                            current_line, current_column, next_char)
                    );
                    errors.push(error);
                    
                    if self.config.continue_on_error {
                        // Skip this character and continue
                        chars.next();
                        current_column += 1;
                    } else {
                        // Break on first error
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
