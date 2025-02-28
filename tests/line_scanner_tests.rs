extern crate rb_tokenizer;

use rb_tokenizer::{Tokenizer, TokenizerConfig};

fn get_line_scanner_tokenizer() -> Tokenizer {
    let config = TokenizerConfig {
        tokenize_whitespace: true,
        continue_on_error: true,
        error_tolerance_limit: 5,
        track_token_positions: true,
    };
    let mut tokenizer = Tokenizer::with_config(config);

    // Add line scanners for different use cases

    // Line comments with //
    tokenizer.add_line_scanner(
        "//",
        "Comment",
        Some("LineComment"),
        true // Include delimiter
    );

    // Preprocessor directive with #
    tokenizer.add_line_scanner(
        "#",
        "Preprocessor",
        None,
        true
    );

    // Custom directive with @
    tokenizer.add_line_scanner(
        "@",
        "Directive",
        None,
        false // Exclude delimiter
    );

    // Add regular scanners for other tokens
    tokenizer.add_regex_scanner(r"^[a-zA-Z_][a-zA-Z0-9_]*", "Identifier", None);
    tokenizer.add_regex_scanner(r"^\d+", "Number", None);
    tokenizer.add_symbol_scanner(";", "Semicolon", None);
    tokenizer.add_symbol_scanner("=", "Operator", Some("Assignment"));
    tokenizer.add_symbol_scanner("+", "Operator", Some("Plus"));

    // Add parentheses scanners for test_mixed_with_regular_code
    tokenizer.add_symbol_scanner("(", "Parenthesis", Some("Open"));
    tokenizer.add_symbol_scanner(")", "Parenthesis", Some("Close"));

    tokenizer
}

#[cfg(test)]
mod line_scanner_tests {
    use super::*;

    #[test]
    fn test_simple_line_comment() {
        let tokenizer = get_line_scanner_tokenizer();

        let input = "// This is a line comment\nvar";
        let result = tokenizer.tokenize(input).expect("Tokenization failed");

        // Expected tokens: LineComment, Whitespace(newline), Identifier
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].token_type, "Comment");
        assert_eq!(result[0].token_sub_type, Some("LineComment".to_string()));
        assert_eq!(result[0].value, "// This is a line comment\n");

        // Check positions
        assert_eq!(result[0].line, 1);
        assert_eq!(result[0].column, 1);
        assert_eq!(result[1].line, 2);
    }

    #[test]
    fn test_line_comment_at_end_of_file() {
        let tokenizer = get_line_scanner_tokenizer();

        // Test with comment at the end of file (no trailing newline)
        let input = "var\n// This is a comment at the end";
        let result = tokenizer.tokenize(input).expect("Tokenization failed");

        // Expected tokens: Identifier, Whitespace(newline), LineComment
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].token_type, "Identifier");
        assert_eq!(result[1].token_type, "Whitespace");
        assert_eq!(result[2].token_type, "Comment");
        assert_eq!(result[2].value, "// This is a comment at the end");

        // The comment should be on line 2
        assert_eq!(result[2].line, 2);
    }

    #[test]
    fn test_preprocessor_directive() {
        let tokenizer = get_line_scanner_tokenizer();

        let input = "#define MAX_SIZE 100\nvar = MAX_SIZE;";
        let result = tokenizer.tokenize(input).expect("Tokenization failed");

        // Expected tokens: Preprocessor, Identifier, Whitespace, Operator, ...
        assert_eq!(result[0].token_type, "Preprocessor");
        assert_eq!(result[0].value, "#define MAX_SIZE 100\n");

        // Should capture the entire directive line
        assert!(result[0].value.contains("define"));
        assert!(result[0].value.contains("MAX_SIZE"));
        assert!(result[0].value.contains("100"));
    }

    #[test]
    fn test_multiple_line_directives() {
        let tokenizer = get_line_scanner_tokenizer();

        let input = "// First comment\n#include <file.h>\n@custom directive";
        let result = tokenizer.tokenize(input).expect("Tokenization failed");

        // Expected tokens: LineComment, Preprocessor, Directive
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].token_type, "Comment");
        assert_eq!(result[1].token_type, "Preprocessor");
        assert_eq!(result[2].token_type, "Directive");

        // Check values
        assert_eq!(result[0].value, "// First comment\n");
        assert_eq!(result[1].value, "#include <file.h>\n");
        // Directive should NOT include the @ delimiter since we configured it that way
        assert_eq!(result[2].value, "custom directive");
    }

    #[test]
    fn test_directive_with_excluded_delimiter() {
        let tokenizer = get_line_scanner_tokenizer();

        let input = "@custom directive\nvar = 10;";
        let result = tokenizer.tokenize(input).expect("Tokenization failed");

        // Expected tokens: Directive, Whitespace(newline), Identifier, ...
        assert_eq!(result[0].token_type, "Directive");
        assert_eq!(result[0].value, "custom directive\n");

        // Verify the delimiter is excluded
        assert!(!result[0].value.starts_with('@'));
    }

    #[test]
    fn test_mixed_with_regular_code() {
        let tokenizer = get_line_scanner_tokenizer();

        let input = "var = 10; // Define variable\n#ifdef DEBUG\nlog(var);\n#endif";
        let result = tokenizer.tokenize(input).expect("Tokenization failed");

        // Verify the line-based tokens were properly extracted
        let comment = result.iter().find(|t| t.token_type == "Comment").unwrap();
        assert_eq!(comment.value, "// Define variable\n");

        let preprocessor1 = result.iter().find(|t| t.token_type == "Preprocessor" && t.value.contains("DEBUG")).unwrap();
        assert_eq!(preprocessor1.value, "#ifdef DEBUG\n");

        let preprocessor2 = result.iter().find(|t| t.token_type == "Preprocessor" && t.value.contains("endif")).unwrap();
        assert_eq!(preprocessor2.value, "#endif");

        // Regular tokens should also be correctly identified
        let identifier = result.iter().find(|t| t.token_type == "Identifier" && t.value == "var").unwrap();
        assert_eq!(identifier.line, 1);
    }

    #[test]
    fn test_line_positions_tracking() {
        let tokenizer = get_line_scanner_tokenizer();

        // Test with multiple line directives to check position tracking
        let input = "// Comment on line 1\n#define on line 2\n@directive on line 3";
        let result = tokenizer.tokenize(input).expect("Tokenization failed");

        // Check line numbers for each token
        assert_eq!(result[0].line, 1);
        assert_eq!(result[0].token_type, "Comment");

        assert_eq!(result[1].line, 2);
        assert_eq!(result[1].token_type, "Preprocessor");

        assert_eq!(result[2].line, 3);
        assert_eq!(result[2].token_type, "Directive");
    }

    #[test]
    fn test_empty_line_directives() {
        let tokenizer = get_line_scanner_tokenizer();

        let input = "//\n#\n@";
        let result = tokenizer.tokenize(input).expect("Tokenization failed");

        // Verify empty directives are recognized
        assert_eq!(result[0].token_type, "Comment");
        assert_eq!(result[0].value, "//\n");

        assert_eq!(result[1].token_type, "Preprocessor");
        assert_eq!(result[1].value, "#\n");

        assert_eq!(result[2].token_type, "Directive");
        assert_eq!(result[2].value, "");
    }

    #[test]
    fn test_adjacent_line_directives() {
        let tokenizer = get_line_scanner_tokenizer();

        // Test with adjacent line directives (no whitespace between them)
        let input = "// First comment\n#include <file.h>\n@directive";
        let result = tokenizer.tokenize(input).expect("Tokenization failed");

        // Check that adjacent directives are properly separated
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].token_type, "Comment");
        assert_eq!(result[1].token_type, "Preprocessor");
        assert_eq!(result[2].token_type, "Directive");

        // Check line positions
        assert_eq!(result[0].line, 1);
        assert_eq!(result[1].line, 2);
        assert_eq!(result[2].line, 3);
    }

    #[test]
    fn test_line_scanner_priority() {
        let tokenizer = get_line_scanner_tokenizer();

        // The # should be recognized as a preprocessor directive, not as a symbol
        let input = "#define";
        let result = tokenizer.tokenize(input).expect("Tokenization failed");

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].token_type, "Preprocessor");
        assert_eq!(result[0].value, "#define");
    }

    #[test]
    fn test_line_scanner_with_special_characters() {
        let tokenizer = get_line_scanner_tokenizer();

        // Test with special characters and escape sequences in the line
        let input = "// Comment with special chars: !@#$%^&*()_+{}|:<>?\n@Directive with \"quotes\" and \\escapes";
        let result = tokenizer.tokenize(input).expect("Tokenization failed");

        assert_eq!(result[0].token_type, "Comment");
        assert!(result[0].value.contains("!@#$%^&*()_+{}|:<>?"));

        assert_eq!(result[1].token_type, "Directive");
        assert!(result[1].value.contains("\"quotes\""));
        assert!(result[1].value.contains("\\escapes"));
    }
}