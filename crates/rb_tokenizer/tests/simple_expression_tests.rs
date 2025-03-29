extern crate rb_tokenizer;

use rb_tokenizer::{Tokenizer, TokenizerConfig};

fn get_tokenizer() -> Tokenizer {
    let config = TokenizerConfig {
        tokenize_whitespace: false,
        continue_on_error: true,
        error_tolerance_limit: 5,
        track_token_positions: true,
    };
    let mut tokenizer = Tokenizer::with_config(config);

    tokenizer.add_regex_scanner(r"^\d+", "Number", None);
    tokenizer.add_regex_scanner(r"^[a-zA-Z_][a-zA-Z0-9_]*", "Identifier", None);
    tokenizer.add_symbol_scanner("(", "Operator", Some("OpenParen"));
    tokenizer.add_symbol_scanner(")", "Operator", Some("CloseParen"));

    // Operators
    tokenizer.add_symbol_scanner("+", "Operator", Some("Plus"));
    tokenizer.add_symbol_scanner("-", "Operator", Some("Minus"));
    tokenizer.add_symbol_scanner("*", "Operator", Some("Multiply"));
    tokenizer.add_symbol_scanner("/", "Operator", Some("Divide"));

    tokenizer
}

#[cfg(test)]
mod tests {
    use crate::get_tokenizer;
    use rb_tokenizer::TokenizerConfig;

    #[test]
    fn it_works() {
        let tokenizer = get_tokenizer();
        let result = tokenizer.tokenize(r"2 + 2 * (3 + 4) - 5");
        match result {
            Ok(tokens) => println!("Tokens: {:?}", tokens),
            Err(errors) => panic!("Tokenization failed: {:?}", errors),
        }
    }
    
    #[test]
    fn test_with_whitespace() {
        let mut tokenizer = get_tokenizer();
        // Modify config to tokenize whitespace
        *tokenizer.config_mut() = TokenizerConfig {
            tokenize_whitespace: true,
            ..tokenizer.config().clone()
        };
        
        let result = tokenizer.tokenize("2 + 2");
        match result {
            Ok(tokens) => {
                println!("Tokens with whitespace: {:?}", tokens);
                // Should have 5 tokens: Number, Whitespace, Plus, Whitespace, Number
                assert_eq!(tokens.len(), 5);
                assert_eq!(tokens[1].token_type, "Whitespace");
                assert_eq!(tokens[3].token_type, "Whitespace");
            },
            Err(errors) => panic!("Tokenization failed: {:?}", errors),
        }
    }
    
    #[test]
    fn test_error_handling() {
        let tokenizer = get_tokenizer();
        // Use an unrecognized character @ which is not defined in our scanners
        let result = tokenizer.tokenize("2 + @ * 5");
        assert!(result.is_err(), "Should return an error for invalid token");
        
        // But with continue_on_error=true, it should parse other valid tokens
        if let Err(errors) = result {
            println!("Expected errors: {:?}", errors);
            // We continue despite errors, so should catch all valid tokens
            // Implement a test for this if needed
        }
    }
}
