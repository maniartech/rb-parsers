use rb_tokenizer::{
    scanners::regex_scanner::RegexScanner,
    scanners::scanner::Scanner,
};

#[cfg(test)]
mod regex_scanner_tests {
    use super::*;

    #[test]
    fn test_basic_regex_scanner() {
        let scanner = RegexScanner::new(r"^\d+", "NUMBER", None);

        // Test matching input
        let result = scanner.scan("123 abc", 0, 0);
        assert!(result.is_ok());
        let token_option = result.unwrap();
        assert!(token_option.is_some());
        let token = token_option.unwrap();
        assert_eq!(token.token_type, "NUMBER");
        assert_eq!(token.value, "123");
        assert_eq!(token.token_sub_type, None);

        // Test non-matching input
        let result = scanner.scan("abc 123", 0, 0);
        assert!(result.is_ok());
        let token_option = result.unwrap();
        assert!(token_option.is_none());
    }

    #[test]
    fn test_regex_scanner_with_subtype() {
        let scanner = RegexScanner::new(r"^(let|const|var)\b", "KEYWORD", Some("DECLARATION"));

        // Test matching inputs with different keywords
        for keyword in ["let", "const", "var"].iter() {
            let test_input = format!("{} x = 5;", keyword);
            let result = scanner.scan(&test_input, 0, 0);
            assert!(result.is_ok());
            let token_option = result.unwrap();
            assert!(token_option.is_some());
            let token = token_option.unwrap();
            assert_eq!(token.token_type, "KEYWORD");
            assert_eq!(token.value, *keyword);
            assert_eq!(token.token_sub_type, Some("DECLARATION".to_string()));
        }
    }

    #[test]
    fn test_regex_scanner_invalid_regex() {
        // This should panic, so we use should_panic attribute
        #[should_panic(expected = "Invalid regex pattern")]
        fn create_invalid_scanner() {
            // Create scanner with invalid regex pattern
            let _scanner = RegexScanner::new(r"[invalid", "INVALID", None);
        }

        create_invalid_scanner();
    }

    #[test]
    fn test_regex_scanner_position_tracking() {
        let scanner = RegexScanner::new(r"^[a-z]+", "WORD", None);

        // Test with specific line and column
        let result = scanner.scan("test 123", 5, 10);
        assert!(result.is_ok());
        let token_option = result.unwrap();
        assert!(token_option.is_some());
        let token = token_option.unwrap();
        assert_eq!(token.token_type, "WORD");
        assert_eq!(token.value, "test");
        assert_eq!(token.line, 5);
        assert_eq!(token.column, 10);
    }

    #[test]
    fn test_regex_scanner_capturing_groups() {
        // Create scanner that uses a regex with capturing groups
        let scanner = RegexScanner::new(r"^(\w+)=(\d+)", "ASSIGNMENT", None);

        // Test with an assignment
        let result = scanner.scan("value=42 next", 0, 0);
        assert!(result.is_ok());
        let token_option = result.unwrap();
        assert!(token_option.is_some());
        let token = token_option.unwrap();
        assert_eq!(token.token_type, "ASSIGNMENT");
        assert_eq!(token.value, "value=42"); // Should capture the entire match
    }
}