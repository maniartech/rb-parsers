use rb_tokenizer::tokens::TokenizationError;

#[cfg(test)]
mod error_tests {
    use super::*;

    #[test]
    fn test_error_creation_and_accessors() {
        let error = TokenizationError::new("Unexpected token", 5, 10);

        assert_eq!(error.message, "Unexpected token");
        assert_eq!(error.line, 5);
        assert_eq!(error.column, 10);
    }

    #[test]
    fn test_error_debug_output() {
        let error = TokenizationError::new("Unclosed string", 7, 15);

        let debug_output = format!("{:?}", error);

        // Check that debug output contains all relevant information
        assert!(debug_output.contains("Unclosed string"));
        assert!(debug_output.contains("7"));
        assert!(debug_output.contains("15"));
    }

    #[test]
    fn test_error_display_output() {
        let error = TokenizationError::new("Invalid character", 3, 8);

        let display_output = format!("{}", error);

        // Check that display output is formatted as expected
        assert!(display_output.contains("Invalid character"));
        assert!(display_output.contains("line 3"));
        assert!(display_output.contains("column 8"));
    }

    #[test]
    fn test_error_equality() {
        let error1 = TokenizationError::new("Syntax error", 10, 20);
        let error2 = TokenizationError::new("Syntax error", 10, 20);
        let different_error = TokenizationError::new("Different error", 10, 20);

        // Test equality
        assert_eq!(error1, error2);
        assert_ne!(error1, different_error);
    }

    #[test]
    fn test_error_with_zero_position() {
        let error = TokenizationError::new("Error at start", 0, 0);

        assert_eq!(error.line, 0);
        assert_eq!(error.column, 0);

        let display_output = format!("{}", error);
        assert!(display_output.contains("line 0"));
        assert!(display_output.contains("column 0"));
    }

    #[test]
    fn test_error_clone() {
        let original = TokenizationError::new("Original error", 100, 200);
        let cloned = original.clone();

        // Verify the cloned error equals the original
        assert_eq!(cloned.message, "Original error");
        assert_eq!(cloned.line, 100);
        assert_eq!(cloned.column, 200);
        assert_eq!(original, cloned);
    }
}