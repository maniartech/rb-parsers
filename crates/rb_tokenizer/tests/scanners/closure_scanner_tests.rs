use rb_tokenizer::{
    tokens::{Token, TokenizationError},
    scanners::closure_scanner::ClosureScanner,
    scanners::scanner::Scanner,
};

#[cfg(test)]
mod closure_scanner_tests {
    use super::*;

    #[test]
    fn test_basic_closure_scanner() {
        let scanner = ClosureScanner::new(Box::new(|input: &str| -> Result<Option<Token>, TokenizationError> {
            if input.starts_with("test") {
                Ok(Some(Token {
                    token_type: "TEST".to_string(),
                    token_sub_type: None,
                    value: "test".to_string(),
                    line: 0,
                    column: 0,
                }))
            } else {
                Ok(None)
            }
        }));

        // Test matching input
        let result = scanner.scan("test_string", 0, 0);
        assert!(result.is_ok());
        let token_option = result.unwrap();
        assert!(token_option.is_some());
        let token = token_option.unwrap();
        assert_eq!(token.token_type, "TEST");
        assert_eq!(token.value, "test");

        // Test non-matching input
        let result = scanner.scan("not_matching", 0, 0);
        assert!(result.is_ok());
        let token_option = result.unwrap();
        assert!(token_option.is_none());
    }

    #[test]
    fn test_closure_scanner_with_error() {
        let scanner = ClosureScanner::new(Box::new(|input: &str| -> Result<Option<Token>, TokenizationError> {
            if input.starts_with("error") {
                Err(TokenizationError::new("Test error", 0, 0))
            } else {
                Ok(None)
            }
        }));

        // Test input that causes an error
        let result = scanner.scan("error_case", 0, 0);
        assert!(result.is_err());
        let error = result.err().unwrap();
        assert_eq!(error.message, "Test error");
    }

    #[test]
    fn test_closure_scanner_with_position_tracking() {
        let scanner = ClosureScanner::new(Box::new(|input: &str| -> Result<Option<Token>, TokenizationError> {
            if input.starts_with("pos") {
                Ok(Some(Token {
                    token_type: "POSITION".to_string(),
                    token_sub_type: None,
                    value: "pos".to_string(),
                    line: 0,  // These will be overwritten by the line/column params
                    column: 0,
                }))
            } else {
                Ok(None)
            }
        }));

        // Test with specific line and column positions
        let result = scanner.scan("pos_test", 10, 20);
        assert!(result.is_ok());
        let token_option = result.unwrap();
        assert!(token_option.is_some());
        let token = token_option.unwrap();
        assert_eq!(token.line, 10);
        assert_eq!(token.column, 20);
    }
}