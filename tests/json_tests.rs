use rb_tokenizer::Tokenizer;

fn get_json_tokenizer() -> Tokenizer {
    let mut tokenizer = Tokenizer::new();

    // Whitespace is implicitly handled by the tokenizer logic

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
        println!("{:?}", result);
    }
}
