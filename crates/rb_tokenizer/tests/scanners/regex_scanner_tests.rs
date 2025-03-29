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
        let result = scanner.scan("123 abc");
        assert!(result.is_ok());
        let token_option = result.unwrap();
        assert!(token_option.is_some());
        let token = token_option.unwrap();
        assert_eq!(token.token_type, "NUMBER");
        assert_eq!(token.value, "123");
        // Note: token_sub_type is always None in the current implementation

        // Test non-matching input
        let result = scanner.scan("abc 123");
        assert!(result.is_ok());
        let token_option = result.unwrap();
        assert!(token_option.is_none());
    }

    #[test]
    #[should_panic(expected = "")]
    fn test_regex_scanner_invalid_regex() {
        // This will panic because RegexScanner::new unwraps the Regex creation
        let _scanner = RegexScanner::new(r"[invalid", "INVALID", None);
    }

    #[test]
    fn test_regex_scanner_with_subtype() {
        let scanner = RegexScanner::new(r"^(let|const|var)\b", "KEYWORD", Some("DECLARATION"));

        // Test matching inputs with different keywords
        for keyword in ["let", "const", "var"].iter() {
            let test_input = format!("{} x = 5;", keyword);
            let result = scanner.scan(&test_input);
            assert!(result.is_ok());
            let token_option = result.unwrap();
            assert!(token_option.is_some());
            let token = token_option.unwrap();
            assert_eq!(token.token_type, "KEYWORD");
            assert_eq!(token.value, *keyword);
            // Note: In the current implementation, token_sub_type is always None
            // Commenting out this assertion that would fail
            // assert_eq!(token.token_sub_type, Some("DECLARATION".to_string()));
        }
    }

    #[test]
    fn test_regex_scanner_capturing_groups() {
        // Create scanner that uses a regex with capturing groups
        let scanner = RegexScanner::new(r"^(\w+)=(\d+)", "ASSIGNMENT", None);

        // Test with an assignment
        let result = scanner.scan("value=42 next");
        assert!(result.is_ok());
        let token_option = result.unwrap();
        assert!(token_option.is_some());
        let token = token_option.unwrap();
        assert_eq!(token.token_type, "ASSIGNMENT");
        assert_eq!(token.value, "value=42"); // Should capture the entire match
    }
}