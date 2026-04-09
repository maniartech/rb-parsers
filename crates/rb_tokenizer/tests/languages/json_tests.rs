use rb_tokenizer::{Tokenizer, TokenizerConfig, WhitespaceScanner};
use rb_tokenizer::scanners::block_scanner::BlockScanner;

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

    // Strings — BlockScanner correctly handles \uXXXX Unicode escapes per the JSON spec.
    let mut string_scanner = BlockScanner::new("\"", "\"", "String", None, false, false, true);
    string_scanner.add_simple_escape('\\');
    string_scanner.add_pattern_escape(r"\\u[0-9a-fA-F]{4}").unwrap();
    tokenizer.add_scanner(Box::new(string_scanner));

    // Numbers — JSON-spec compliant: no leading zeros, optional fraction and exponent.
    // The leading minus is part of the number literal per the JSON grammar.
    tokenizer.add_regex_scanner(
        r"^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?",
        "Number",
        None,
    ).unwrap();

    // Literals
    tokenizer.add_keyword_scanner("Literal", &["true", "false", "null"]);

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
        assert_eq!((result[0].display_line(), result[0].display_column()), (1, 1));
        assert_eq!((result[1].display_line(), result[1].display_column()), (2, 13));
    }

    #[test]
    fn test_json_with_whitespace_tokens() {
        // Use an explicit WhitespaceScanner instead of the built-in config flag.
        // WhitespaceScanner::uniform emits all whitespace as a single "Whitespace" token,
        // which is correct for JSON where all whitespace is insignificant.
        let mut tokenizer = get_json_tokenizer();
        tokenizer.add_whitespace_scanner(WhitespaceScanner::uniform("Whitespace"));

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

    #[test]
    fn test_json_numbers() {
        let tokenizer = get_json_tokenizer();
        for (input, expected) in [
            ("-42",     "-42"),
            ("3.14",    "3.14"),
            ("6.02e23", "6.02e23"),
            ("-1.5e-3", "-1.5e-3"),
            ("0",       "0"),
            ("1e+10",   "1e+10"),
        ] {
            let result = tokenizer.tokenize(input).unwrap();
            assert_eq!(result.len(), 1, "Expected one Number token for '{input}'");
            assert_eq!(result[0].token_type, "Number");
            assert_eq!(result[0].value, expected);
        }
    }

    #[test]
    fn test_json_string_unicode_escapes() {
        let tokenizer = get_json_tokenizer();
        // \u0041 = 'A', \u0042 = 'B'
        let input = r#""\u0041\u0042""#;
        let result = tokenizer.tokenize(input).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].token_type, "String");
        // Raw escape sequences are preserved; Unicode decoding is the parser's job.
        assert!(result[0].value.contains(r"\u0041"));
        assert!(result[0].value.contains(r"\u0042"));
    }
}
