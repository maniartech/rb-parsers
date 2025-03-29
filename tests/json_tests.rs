use rb_tokenizer::{Tokenizer, TokenizerConfig};

fn get_json_tokenizer() -> Tokenizer {
    let config = TokenizerConfig {
        tokenize_whitespace: false,
        continue_on_error: false,
        error_tolerance_limit: 1,
        track_token_positions: true,
    };
    let mut tokenizer = Tokenizer::with_config(config);

    tokenizer.add_symbol_scanner("{", "Brace", Some("OpenBrace"));
    tokenizer.add_symbol_scanner("}", "Brace", Some("CloseBrace"));
    tokenizer.add_symbol_scanner("[", "Bracket", Some("OpenBracket"));
    tokenizer.add_symbol_scanner("]", "Bracket", Some("CloseBracket"));
    tokenizer.add_symbol_scanner(":", "Colon", None);
    tokenizer.add_symbol_scanner(",", "Comma", None);

    tokenizer.add_regex_scanner(r#"^"([^"\\]|\\.)*""#, "String", None);
    tokenizer.add_regex_scanner(r"^-?\d+(\.\d+)?([eE][-+]?\d+)?", "Number", None);
    tokenizer.add_regex_scanner(r"^(true|false|null)\b", "Literal", None);

    tokenizer
}

#[cfg(test)]
mod json_tests {
    use rb_tokenizer::utils::pretty_print_tokens;

    use super::*;

    #[test]
    fn test_json_tokenization() {
        let tokenizer = get_json_tokenizer();
        let json_input = r#"{
            "key": "value",
            "array": [true, 123, null]
        }"#;
        let result = tokenizer.tokenize(json_input).expect("Tokenization failed");

        assert_eq!(result.len(), 15, "Unexpected number of tokens");
        println!("JSON tokens: {:?}", result);
    }

    #[test]
    fn test_json_with_whitespace_tokens() {
        let mut tokenizer = get_json_tokenizer();
        *tokenizer.config_mut() = TokenizerConfig {
            tokenize_whitespace: true,
            ..tokenizer.config().clone()
        };

        let json_input = r#"{"key": "value"}"#;
        let result = tokenizer.tokenize(json_input).expect("Tokenization failed");

        pretty_print_tokens(&result);
        assert_eq!(result.len(), 6, "Unexpected number of tokens when whitespace is included");
        assert_eq!(result[3].token_type, "Whitespace");
        assert_eq!(result[3].value, " ");
        println!("JSON tokens with whitespace: {:?}", result);
    }

    #[test]
    fn test_json_error_handling() {
        let tokenizer = get_json_tokenizer();
        let invalid_json = r#"{"key": @value}"#;
        let result = tokenizer.tokenize(invalid_json);

        assert!(result.is_err(), "Should return an error for invalid token");
        if let Err(errors) = result {
            println!("Expected JSON parsing errors: {:?}", errors);
            assert!(!errors.is_empty(), "Should contain at least one error");
        }
    }
}