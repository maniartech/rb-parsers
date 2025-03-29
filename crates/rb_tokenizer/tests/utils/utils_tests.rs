use rb_tokenizer::{tokens::Token, utils};
use std::collections::HashMap;

#[cfg(test)]
mod utils_tests {
    use super::*;

    fn create_test_tokens() -> Vec<Token> {
        vec![
            Token {
                token_type: "IDENTIFIER".to_string(),
                token_sub_type: Some("VARIABLE".to_string()),
                value: "myVar".to_string(),
                line: 1,
                column: 5,
            },
            Token {
                token_type: "OPERATOR".to_string(),
                token_sub_type: Some("ASSIGNMENT".to_string()),
                value: "=".to_string(),
                line: 1,
                column: 11,
            },
            Token {
                token_type: "NUMBER".to_string(),
                token_sub_type: None,
                value: "42".to_string(),
                line: 1,
                column: 13,
            },
            Token {
                token_type: "PUNCTUATION".to_string(),
                token_sub_type: Some("SEMICOLON".to_string()),
                value: ";".to_string(),
                line: 1,
                column: 15,
            },
            Token {
                token_type: "WHITESPACE".to_string(),
                token_sub_type: Some("NEWLINE".to_string()),
                value: "\n".to_string(),
                line: 1,
                column: 16,
            },
        ]
    }

    #[test]
    fn test_pretty_print_tokens() {
        let tokens = create_test_tokens();

        // Test pretty printing
        let output = utils::pretty_print_tokens(&tokens);

        // Verify the output contains information about each token
        assert!(output.contains("Tokens:"));
        assert!(output.contains("IDENTIFIER"));
        assert!(output.contains("myVar"));
        assert!(output.contains("VARIABLE"));
        assert!(output.contains("OPERATOR"));
        assert!(output.contains("ASSIGNMENT"));
        assert!(output.contains("NUMBER"));
        assert!(output.contains("42"));
        assert!(output.contains("line 1"));
    }

    #[test]
    fn test_token_summary() {
        let token = Token {
            token_type: "STRING".to_string(),
            token_sub_type: Some("DOUBLE_QUOTED".to_string()),
            value: "Hello\nWorld".to_string(),
            line: 2,
            column: 3,
        };

        let summary = utils::token_summary(&token);

        // Verify the summary format
        assert!(summary.contains("STRING"));
        assert!(summary.contains("DOUBLE_QUOTED"));
        assert!(summary.contains("Hello\\nWorld")); // Newlines should be escaped
    }

    #[test]
    fn test_compare_tokens() {
        let expected = create_test_tokens();
        let mut actual = expected.clone();

        // Modify one token in the actual result to create a difference
        if !actual.is_empty() {
            actual[0].value = "differentVar".to_string();
        }

        // Get comparison output
        let comparison = utils::compare_tokens(&expected, &actual);

        // Verify comparison contains all tokens
        assert!(comparison.contains("Token Comparison:"));
        assert!(comparison.contains("Expected"));
        assert!(comparison.contains("Actual"));
        assert!(comparison.contains("myVar"));
        assert!(comparison.contains("differentVar"));
        assert!(comparison.contains("✗")); // Should show a difference marker
        assert!(comparison.contains("✓")); // Should show some matches too
    }

    #[test]
    fn test_visualize_token_positions() {
        let input = "let myVar = 42;\n";
        let tokens = create_test_tokens();

        let visualization = utils::visualize_token_positions(input, &tokens);

        // Verify visualization format
        assert!(visualization.contains("let myVar = 42;"));
        assert!(visualization.contains("^")); // Should contain position markers
        assert!(visualization.contains("1 | ")); // Should show line numbers
    }

    #[test]
    fn test_analyze_tokens() {
        let tokens = create_test_tokens();

        let analysis = utils::analyze_tokens(&tokens);

        // Verify analysis content
        assert!(analysis.contains("Token Analysis:"));
        assert!(analysis.contains("Total Tokens: 5"));
        assert!(analysis.contains("Token Type Distribution:"));
        assert!(analysis.contains("IDENTIFIER"));
        assert!(analysis.contains("NUMBER"));
        assert!(analysis.contains("Potential Issues:"));
    }
}