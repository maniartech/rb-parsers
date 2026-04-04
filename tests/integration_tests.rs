// filepath: rust-monorepo/tests/integration_tests.rs
#[cfg(test)]
mod integration_tests {
    use rb_tokenizer::Tokenizer;
    use rb_tokenizer::TokenizerConfig;

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
        tokenizer.add_regex_scanner(r#"^"([^"\\]|\\.)*""#, "String", None).unwrap();
        tokenizer.add_regex_scanner(r"^-?\d+(\.\d+)?([eE][-+]?\d+)?", "Number", None).unwrap();
        tokenizer.add_regex_scanner(r"^(true|false|null)\b", "Literal", None).unwrap();

        tokenizer
    }

    #[test]
    fn test_workspace_json_tokenization() {
        let tokenizer = get_json_tokenizer();
        let json_input = r#"{"key": "value"}"#;
        let result = tokenizer.tokenize(json_input).expect("Tokenization failed");

        assert_eq!(result.len(), 5, "Unexpected number of tokens");
    }
}