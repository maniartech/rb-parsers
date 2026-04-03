use rb_tokenizer::tokens::TokenizationError;
use std::error::Error;

#[cfg(test)]
mod error_tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let error = TokenizationError::UnrecognizedToken("Unexpected token".to_string());

        // Verify the error is created correctly
        match error {
            TokenizationError::UnrecognizedToken(msg) => {
                assert_eq!(msg, "Unexpected token");
            },
            _ => panic!("Expected UnrecognizedToken variant")
        }
    }

    #[test]
    fn test_error_debug_output() {
        let error = TokenizationError::UnrecognizedToken("Unclosed string".to_string());

        let debug_output = format!("{:?}", error);

        // Check that debug output contains relevant information
        assert!(debug_output.contains("UnrecognizedToken"));
        assert!(debug_output.contains("Unclosed string"));
    }

    #[test]
    fn test_error_display_output() {
        let error = TokenizationError::UnrecognizedToken("Invalid character".to_string());

        let display_output = format!("{}", error);

        // Check that display output is formatted as expected
        assert!(display_output.contains("Unrecognized token"));
        assert!(display_output.contains("Invalid character"));
    }

    #[test]
    fn test_error_variants_distinct() {
        let error1 = TokenizationError::UnrecognizedToken("Syntax error".to_string());
        let error2 = TokenizationError::UnrecognizedToken("Syntax error".to_string());
        let different_error = TokenizationError::UnrecognizedToken("Different error".to_string());
        let block_error = TokenizationError::UnmatchedBlockDelimiter("{{".to_string(), "}}".to_string());

        // Check variant type and content instead of equality
        match (&error1, &error2) {
            (TokenizationError::UnrecognizedToken(msg1), TokenizationError::UnrecognizedToken(msg2)) => {
                assert_eq!(msg1, msg2, "Messages should be equal");
            },
            _ => panic!("Both should be UnrecognizedToken variant")
        }

        // Check that different messages are reflected
        match (&error1, &different_error) {
            (TokenizationError::UnrecognizedToken(msg1), TokenizationError::UnrecognizedToken(msg2)) => {
                assert_ne!(msg1, msg2, "Messages should be different");
            },
            _ => panic!("Both should be UnrecognizedToken variant")
        }

        // Check that different variants are reflected
        if let TokenizationError::UnmatchedBlockDelimiter(_, _) = block_error {
            // This is correct
        } else {
            panic!("Expected UnmatchedBlockDelimiter variant");
        }
    }

    #[test]
    fn test_error_clone() {
        let original = TokenizationError::UnrecognizedToken("Original error".to_string());
        let cloned = original.clone();

        // Verify the cloned error matches the original
        match (original, cloned) {
            (TokenizationError::UnrecognizedToken(msg1), TokenizationError::UnrecognizedToken(msg2)) => {
                assert_eq!(msg1, msg2, "Cloned message should match original");
            },
            _ => panic!("Both should be UnrecognizedToken variant")
        }
    }

    #[test]
    fn test_error_trait_implementation() {
        let error = TokenizationError::UnrecognizedToken("Test error".to_string());

        // Test that our error implements the Error trait
        let error_trait_object: &dyn Error = &error;
        let description = error_trait_object.to_string();

        assert!(description.contains("Unrecognized token"));
    }
}