use rb_tokenizer::tokens::{Token, SourceId, SourcePosition, SourceSpan};

#[cfg(test)]
mod token_tests {
    use super::*;

    #[test]
    fn test_token_creation_and_accessors() {
        let token = Token {
            token_type: "IDENTIFIER",
            token_sub_type: Some("VARIABLE"),
            value: "myVariable".to_string(),
            span: SourceSpan {
                source_id: SourceId::UNKNOWN,
                start: SourcePosition { byte_offset: 0, line: 41, column: 9 },
                end: SourcePosition::ZERO,
            },
        };

        // Test basic properties
        assert_eq!(token.token_type, "IDENTIFIER");
        assert_eq!(token.token_sub_type, Some("VARIABLE"));
        assert_eq!(token.value, "myVariable");
        assert_eq!(token.display_line(), 42);
        assert_eq!(token.display_column(), 10);
    }

    #[test]
    fn test_token_with_no_subtype() {
        let token = Token {
            token_type: "NUMBER",
            token_sub_type: None,
            value: "123.45".to_string(),
            span: SourceSpan::UNKNOWN,
        };

        assert_eq!(token.token_type, "NUMBER");
        assert_eq!(token.token_sub_type, None);
        assert_eq!(token.value, "123.45");
    }

    #[test]
    fn test_token_equality() {
        let token1 = Token {
            token_type: "KEYWORD",
            token_sub_type: Some("CONTROL"),
            value: "if".to_string(),
            span: SourceSpan::UNKNOWN,
        };

        let token2 = Token {
            token_type: "KEYWORD",
            token_sub_type: Some("CONTROL"),
            value: "if".to_string(),
            span: SourceSpan::UNKNOWN,
        };

        let different_token = Token {
            token_type: "KEYWORD",
            token_sub_type: Some("CONTROL"),
            value: "else".to_string(),
            span: SourceSpan::UNKNOWN,
        };

        // Test equality
        assert_eq!(token1, token2);
        assert_ne!(token1, different_token);
    }

    #[test]
    fn test_token_clone() {
        let original = Token {
            token_type: "STRING",
            token_sub_type: Some("DOUBLE_QUOTED"),
            value: "Hello, world!".to_string(),
            span: SourceSpan {
                source_id: SourceId::UNKNOWN,
                start: SourcePosition { byte_offset: 0, line: 6, column: 11 },
                end: SourcePosition::ZERO,
            },
        };

        let cloned = original.clone();

        // Verify the cloned token is equal but not the same instance
        assert_eq!(original, cloned);
        assert_eq!(cloned.token_type, "STRING");
        assert_eq!(cloned.token_sub_type, Some("DOUBLE_QUOTED"));
        assert_eq!(cloned.value, "Hello, world!");
        assert_eq!(cloned.display_line(), 7);
        assert_eq!(cloned.display_column(), 12);
    }

    #[test]
    fn test_token_debug_output() {
        let token = Token {
            token_type: "OPERATOR",
            token_sub_type: Some("ARITHMETIC"),
            value: "+".to_string(),
            span: SourceSpan {
                source_id: SourceId::UNKNOWN,
                start: SourcePosition { byte_offset: 0, line: 14, column: 7 },
                end: SourcePosition::ZERO,
            },
        };

        // Test Debug implementation
        let debug_output = format!("{:?}", token);

        // Verify debug output contains all relevant information
        assert!(debug_output.contains("OPERATOR"));
        assert!(debug_output.contains("ARITHMETIC"));
        assert!(debug_output.contains("+"));
        assert_eq!(token.display_line(), 15);
        assert_eq!(token.display_column(), 8);
    }

    #[test]
    fn test_token_with_multiline_content() {
        let token = Token {
            token_type: "COMMENT",
            token_sub_type: Some("BLOCK"),
            value: "/* This is\na multiline\ncomment */".to_string(),
            span: SourceSpan::UNKNOWN,
        };

        assert_eq!(token.token_type, "COMMENT");
        assert_eq!(token.value, "/* This is\na multiline\ncomment */");

        // Count newlines in the token value
        let newline_count = token.value.chars().filter(|&c| c == '\n').count();
        assert_eq!(newline_count, 2);
    }
}