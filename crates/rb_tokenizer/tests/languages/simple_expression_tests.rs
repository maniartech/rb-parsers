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
    use super::get_tokenizer;

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

        // Use fluent API instead of direct config manipulation
        tokenizer.set_tokenize_whitespace(true);

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
        // First test with continue_on_error = false
        let mut strict_tokenizer = get_tokenizer();
        strict_tokenizer.set_continue_on_error(false);

        // Use an unrecognized character @ which is not defined in our scanners
        let strict_result = strict_tokenizer.tokenize("2 + @ * 5");

        // Should return an error in strict mode for invalid token
        assert!(strict_result.is_err(), "Should return an error in strict mode for invalid token");

        if let Err(errors) = strict_result {
            println!("Expected errors in strict mode: {:?}", errors);
            assert!(!errors.is_empty(), "Should have at least one error");
        }

        // Now test with continue_on_error = true (default for get_tokenizer())
        let tolerant_tokenizer = get_tokenizer();
        let tolerant_result = tolerant_tokenizer.tokenize("2 + @ * 5");

        // With continue_on_error=true, it should return Ok with tokens
        assert!(tolerant_result.is_ok(), "Should return Ok with tokens in tolerant mode");

        // But it should also store the errors in last_errors
        let errors = tolerant_tokenizer.last_errors();
        assert!(errors.is_some(), "Should have stored errors in last_errors");

        if let Some(error_list) = errors {
            println!("Errors in tolerant mode: {:?}", error_list);
            assert!(!error_list.is_empty(), "Should have at least one error");
            assert!(error_list[0].to_string().contains("@"),
                   "Error should mention the problematic character @");
        }

        // Check that we got the valid tokens even with errors
        if let Ok(tokens) = tolerant_result {
            println!("Tokens parsed in tolerant mode: {:?}", tokens);
            assert!(tokens.len() >= 3, "Should have parsed at least 3 valid tokens");

            // Check we got the expected token types
            let token_types: Vec<_> = tokens.iter().map(|t| &t.token_type).collect();
            println!("Token types: {:?}", token_types);

            // Should have "Number" and "Operator" tokens at minimum
            assert!(token_types.contains(&&"Number".to_string()), "Should have Number token");
            assert!(token_types.contains(&&"Operator".to_string()), "Should have Operator token");
        }
    }
}
