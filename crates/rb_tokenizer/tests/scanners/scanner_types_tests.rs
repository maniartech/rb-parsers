use rb_tokenizer::{
    tokens::{Token, TokenizationError},
    scanners::scanner_types::{ScannerType, CallbackScanner},
    scanners::symbol_scanner::SymbolScanner,
    scanners::regex_scanner::RegexScanner,
    scanners::scanner::Scanner,
};

#[cfg(test)]
mod scanner_types_tests {
    use super::*;

    #[test]
    fn test_scanner_type_symbol() {
        let symbol_scanner = SymbolScanner::new("if", "KEYWORD", None);
        let scanner_type = ScannerType::Symbol(symbol_scanner);

        // Test as a scanner
        let result = scanner_type.scan("if condition");
        assert!(result.is_ok());
        let token_option = result.unwrap();
        assert!(token_option.is_some());
        let token = token_option.unwrap();
        assert_eq!(token.token_type, "KEYWORD");
        assert_eq!(token.value, "if");
    }

    #[test]
    fn test_scanner_type_regex() {
        let regex_scanner = RegexScanner::new(r"^\d+", "NUMBER", None);
        let scanner_type = ScannerType::Regex(regex_scanner);

        // Test as a scanner
        let result = scanner_type.scan("123 abc");
        assert!(result.is_ok());
        let token_option = result.unwrap();
        assert!(token_option.is_some());
        let token = token_option.unwrap();
        assert_eq!(token.token_type, "NUMBER");
        assert_eq!(token.value, "123");
    }

    #[test]
    fn test_scanner_type_closure() {
        struct TestCallbackScanner;

        impl CallbackScanner for TestCallbackScanner {
            fn scan(&self, input: &str) -> Result<Option<Token>, TokenizationError> {
                if input.starts_with("test") {
                    Ok(Some(Token {
                        token_type: "TEST",
                        token_sub_type: None,
                        value: "test".to_string(),
                        line: 0,
                        column: 0,
                    }))
                } else {
                    Ok(None)
                }
            }
        }

        let callback_scanner = Box::new(TestCallbackScanner);
        let scanner_type = ScannerType::Callback(callback_scanner);

        // Test with matching input
        let result = scanner_type.scan("test_input");
        assert!(result.is_ok());
        let token_option = result.unwrap();
        assert!(token_option.is_some());
        let token = token_option.unwrap();
        assert_eq!(token.token_type, "TEST");
        assert_eq!(token.value, "test");

        // Test with non-matching input
        let result = scanner_type.scan("other input");
        assert!(result.is_ok());
        let token_option = result.unwrap();
        assert!(token_option.is_none());
    }

    #[test]
    fn test_scanner_type_error_handling() {
        struct ErrorCallbackScanner;

        impl CallbackScanner for ErrorCallbackScanner {
            fn scan(&self, input: &str) -> Result<Option<Token>, TokenizationError> {
                if input.starts_with("error") {
                    Err(TokenizationError::UnrecognizedToken("Test error".to_string()))
                } else {
                    Ok(None)
                }
            }
        }

        let error_scanner = Box::new(ErrorCallbackScanner);
        let scanner_type = ScannerType::Callback(error_scanner);

        // Test with error-inducing input
        let result = scanner_type.scan("error_case");
        assert!(result.is_err());

        // Check the error type
        match result.err().unwrap() {
            TokenizationError::UnrecognizedToken(msg) => {
                assert_eq!(msg, "Test error");
            },
            _ => panic!("Expected UnrecognizedToken error")
        }
    }
}