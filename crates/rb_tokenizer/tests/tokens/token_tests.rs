use rb_tokenizer::tokens::Token;

#[cfg(test)]
mod token_tests {
    use super::*;

    #[test]
    fn test_token_creation_and_accessors() {
        let token = Token {
            token_type: "IDENTIFIER".to_string(),
            token_sub_type: Some("VARIABLE".to_string()),
            value: "myVariable".to_string(),
            line: 42,
            column: 10,
        };

        // Test basic properties
        assert_eq!(token.token_type, "IDENTIFIER");
        assert_eq!(token.token_sub_type, Some("VARIABLE".to_string()));
        assert_eq!(token.value, "myVariable");
        assert_eq!(token.line, 42);
        assert_eq!(token.column, 10);
    }

    #[test]
    fn test_token_with_no_subtype() {
        let token = Token {
            token_type: "NUMBER".to_string(),
            token_sub_type: None,
            value: "123.45".to_string(),
            line: 5,
            column: 20,
        };

        assert_eq!(token.token_type, "NUMBER");
        assert_eq!(token.token_sub_type, None);
        assert_eq!(token.value, "123.45");
    }

    #[test]
    fn test_token_equality() {
        let token1 = Token {
            token_type: "KEYWORD".to_string(),
            token_sub_type: Some("CONTROL".to_string()),
            value: "if".to_string(),
            line: 10,
            column: 5,
        };

        let token2 = Token {
            token_type: "KEYWORD".to_string(),
            token_sub_type: Some("CONTROL".to_string()),
            value: "if".to_string(),
            line: 10,
            column: 5,
        };

        let different_token = Token {
            token_type: "KEYWORD".to_string(),
            token_sub_type: Some("CONTROL".to_string()),
            value: "else".to_string(),
            line: 10,
            column: 15,
        };

        // Test equality
        assert_eq!(token1, token2);
        assert_ne!(token1, different_token);
    }

    #[test]
    fn test_token_clone() {
        let original = Token {
            token_type: "STRING".to_string(),
            token_sub_type: Some("DOUBLE_QUOTED".to_string()),
            value: "Hello, world!".to_string(),
            line: 7,
            column: 12,
        };

        let cloned = original.clone();

        // Verify the cloned token is equal but not the same instance
        assert_eq!(original, cloned);
        assert_eq!(cloned.token_type, "STRING");
        assert_eq!(cloned.token_sub_type, Some("DOUBLE_QUOTED".to_string()));
        assert_eq!(cloned.value, "Hello, world!");
        assert_eq!(cloned.line, 7);
        assert_eq!(cloned.column, 12);
    }

    #[test]
    fn test_token_debug_output() {
        let token = Token {
            token_type: "OPERATOR".to_string(),
            token_sub_type: Some("ARITHMETIC".to_string()),
            value: "+".to_string(),
            line: 15,
            column: 8,
        };

        // Test Debug implementation
        let debug_output = format!("{:?}", token);

        // Verify debug output contains all relevant information
        assert!(debug_output.contains("OPERATOR"));
        assert!(debug_output.contains("ARITHMETIC"));
        assert!(debug_output.contains("+"));
        assert!(debug_output.contains("15"));
        assert!(debug_output.contains("8"));
    }

    #[test]
    fn test_token_with_multiline_content() {
        let token = Token {
            token_type: "COMMENT".to_string(),
            token_sub_type: Some("BLOCK".to_string()),
            value: "/* This is\na multiline\ncomment */".to_string(),
            line: 20,
            column: 0,
        };

        assert_eq!(token.token_type, "COMMENT");
        assert_eq!(token.value, "/* This is\na multiline\ncomment */");

        // Count newlines in the token value
        let newline_count = token.value.chars().filter(|&c| c == '\n').count();
        assert_eq!(newline_count, 2);
    }
}