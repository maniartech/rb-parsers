use rb_tokenizer::{Tokenizer, TokenizerConfig};

fn get_json_tokenizer() -> Tokenizer {
    let config = TokenizerConfig {
        tokenize_whitespace: false,
        continue_on_error: false, // For JSON we want strict parsing
        error_tolerance_limit: 1,
        track_token_positions: true,
    };
    let mut tokenizer = Tokenizer::with_config(config);

    // Structural characters
    tokenizer.add_symbol_scanner("{", "Brace", Some("OpenBrace"));
    tokenizer.add_symbol_scanner("}", "Brace", Some("CloseBrace"));
    tokenizer.add_symbol_scanner("[", "Bracket", Some("OpenBracket"));
    tokenizer.add_symbol_scanner("]", "Bracket", Some("CloseBracket"));
    tokenizer.add_symbol_scanner(":", "Colon", None);
    tokenizer.add_symbol_scanner(",", "Comma", None);

    // Strings
    tokenizer.add_regex_scanner(r#"^"([^"\\]|\\.)*""#, "String", None).unwrap();

    // Numbers
    tokenizer.add_regex_scanner(r"^-?\d+(\.\d+)?([eE][-+]?\d+)?", "Number", None).unwrap();

    // Literals
    tokenizer.add_regex_scanner(r"^(true|false|null)\b", "Literal", None).unwrap();

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

        let actual: Vec<_> = result.iter()
            .map(|token| (token.token_type, token.token_sub_type, token.value.as_str()))
            .collect();

        let expected = vec![
            ("Brace", Some("OpenBrace"), "{"),
            ("String", None, "\"key\""),
            ("Colon", None, ":"),
            ("String", None, "\"value\""),
            ("Comma", None, ","),
            ("String", None, "\"array\""),
            ("Colon", None, ":"),
            ("Bracket", Some("OpenBracket"), "["),
            ("Literal", None, "true"),
            ("Comma", None, ","),
            ("Number", None, "123"),
            ("Comma", None, ","),
            ("Literal", None, "null"),
            ("Bracket", Some("CloseBracket"), "]"),
            ("Brace", Some("CloseBrace"), "}"),
        ];

        assert_eq!(actual, expected);
        assert_eq!((result[0].line, result[0].column), (1, 1));
        assert_eq!((result[1].line, result[1].column), (2, 13));
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

        pretty_print_tokens(&result);

        let actual: Vec<_> = result.iter()
            .map(|token| (token.token_type, token.token_sub_type, token.value.as_str()))
            .collect();

        let expected = vec![
            ("Brace", Some("OpenBrace"), "{"),
            ("String", None, "\"key\""),
            ("Colon", None, ":"),
            ("Whitespace", None, " "),
            ("String", None, "\"value\""),
            ("Brace", Some("CloseBrace"), "}"),
        ];

        assert_eq!(actual, expected);
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
