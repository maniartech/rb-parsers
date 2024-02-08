use crate::rules::{self, RegexRule, Rule, RuleType, SymbolRule};
use crate::tokens::{Token, TokenizationError};
pub struct Tokenizer {
    rules: Vec<RuleType>,
}

impl Tokenizer {
    pub fn new() -> Self {
        Tokenizer { rules: Vec::new() }
    }

    pub fn add_rule(&mut self, rule: Box<dyn rules::Rule>) {
        self.rules.push(RuleType::Rule(rule));
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

    // Adjust the tokenize method to handle the Option for default_rule
    pub fn tokenize(&self, input: &str) -> Result<Vec<Token>, Vec<TokenizationError>> {
        let mut tokens = Vec::new();
        let mut errors = Vec::new();

        let mut current_line = 1;
        let mut current_column = 1; // Start column counting from 1
        let mut chars = input.char_indices().peekable();

        while let Some((start, next_char)) = chars.peek().copied() {
            let mut matched = false;

            // Calculate whitespace and update column if necessary
            if next_char.is_whitespace() {
                if next_char == '\n' {
                    current_line += 1;
                    current_column = 1; // Reset column at new line
                } else {
                    current_column += 1; // Increment column for spaces
                }
                chars.next(); // Consume the whitespace character
                continue; // Move to the next character
            }

            let current_input = &input[start..];

            for rule in &self.rules {
                match rule.process(current_input) {
                    Ok(Some(token)) => {
                        let token_len = token.value.len();
                        tokens.push(Token {
                            line: current_line,
                            column: current_column, // Use current column for the token
                            ..token
                        });
                        // Advance the iterator by token_len characters and update column accordingly
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
                        break; // Break to process the next segment of input
                    }
                    Ok(None) => {}            // No match, continue to next rule
                    Err(e) => errors.push(e), // Error encountered
                }
            }

            if !matched {
                // If no rules matched, consider handling or reporting it
                errors.push(TokenizationError::UnrecognizedToken(
                    current_input.to_string(),
                ));
                break; // Or handle it differently based on your needs
            }
        }

        if errors.is_empty() {
            Ok(tokens)
        } else {
            Err(errors)
        }
    }
}
