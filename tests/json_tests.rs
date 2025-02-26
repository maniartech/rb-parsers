use rb_tokenizer::{Tokenizer, TokenizerConfig};

fn get_json_tokenizer() -> Tokenizer {
    let config = TokenizerConfig {
        tokenize_whitespace: false,
        continue_on_error: false, // For JSON we want strict parsing
        error_tolerance_limit: 1,
    };
    let mut tokenizer = Tokenizer::with_config(config);

    // Structural characters
    tokenizer.add_symbol_rule("{", "Brace", Some("OpenBrace"));
    tokenizer.add_symbol_rule("}", "Brace", Some("CloseBrace"));
    tokenizer.add_symbol_rule("[", "Bracket", Some("OpenBracket"));
    tokenizer.add_symbol_rule("]", "Bracket", Some("CloseBracket"));
    tokenizer.add_symbol_rule(":", "Colon", None);
    tokenizer.add_symbol_rule(",", "Comma", None);

    // Strings
    tokenizer.add_regex_rule(r#"^"([^"\\]|\\.)*""#, "String", None);

    // Numbers
    tokenizer.add_regex_rule(r"^-?\d+(\.\d+)?([eE][-+]?\d+)?", "Number", None);

    // Literals
    tokenizer.add_regex_rule(r"^(true|false|null)\b", "Literal", None);

    tokenizer
}

#[cfg(test)]
mod json_tests {
    use super::*;

    #[test]
    fn test_json_tokenization() {
        let tokenizer = get_json_tokenizer();
        let json_input = r#"{
            "key": "value",
            "array": [true, 123, null]
        }"#;
        let result = tokenizer.tokenize(json_input).expect("Tokenization failed");

        // Expected tokens: OpenBrace, String, Colon, String, Comma, String, Colon, OpenBracket, Literal, Comma, Number, Comma, Literal, CloseBracket, CloseBrace
        assert_eq!(result.len(), 15, "Unexpected number of tokens");

        // This is a basic check. For a thorough test, you should verify each token's type, value, and possibly positions.
        println!("JSON tokens: {:?}", result);
    }
    
    #[test]
    fn test_json_with_whitespace_tokens() {
        let mut tokenizer = get_json_tokenizer();
        // Modify config to tokenize whitespace
        *tokenizer.config_mut() = TokenizerConfig {
            tokenize_whitespace: true,
            ..tokenizer.config().clone()
        };
        
        let json_input = r#"{"key": "value"}"#;
        let result = tokenizer.tokenize(json_input).expect("Tokenization failed");
        
        // Expected tokens with whitespace included: OpenBrace, String, Colon, Whitespace, String, CloseBrace
        assert_eq!(result.len(), 6, "Unexpected number of tokens when whitespace is included");
        
        // Verify the whitespace token
        assert_eq!(result[3].token_type, "Whitespace");
        assert_eq!(result[3].value, " ");
        
        println!("JSON tokens with whitespace: {:?}", result);
    }
    
    #[test]
    fn test_json_error_handling() {
        let tokenizer = get_json_tokenizer();
        
        // Invalid JSON with an unrecognized token
        let invalid_json = r#"{"key": @value}"#;
        let result = tokenizer.tokenize(invalid_json);
        
        assert!(result.is_err(), "Should return an error for invalid token");
        if let Err(errors) = result {
            println!("Expected JSON parsing errors: {:?}", errors);
            assert!(!errors.is_empty(), "Should contain at least one error");
        }
    }
}
