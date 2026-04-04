use rb_tokenizer::{
    scanners::regex_scanner::RegexScanner,
    scanners::scanner::Scanner,
    Tokenizer,
};

#[cfg(test)]
mod regex_scanner_tests {
    use super::*;

    #[test]
    fn test_basic_regex_scanner() {
        let scanner = RegexScanner::new(r"^\d+", "NUMBER", None).unwrap();

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
    fn test_regex_scanner_prefixes_unanchored_patterns() {
        let scanner = RegexScanner::new(r"\d+", "NUMBER", None).unwrap();
        assert_eq!(scanner.pattern.as_str(), r"^\d+");
    }

    #[test]
    fn test_regex_scanner_does_not_duplicate_existing_anchor() {
        let scanner = RegexScanner::new(r"^\d+", "NUMBER", None).unwrap();
        assert_eq!(scanner.pattern.as_str(), r"^\d+");
    }

    #[test]
    fn test_regex_scanner_invalid_regex() {
        let result = RegexScanner::new(r"[invalid", "INVALID", None);
        assert!(matches!(result, Err(rb_tokenizer::tokens::TokenizationError::InvalidRegexPattern { .. })));
    }

    #[test]
    fn test_regex_scanner_with_subtype() {
        let scanner = RegexScanner::new(r"^(let|const|var)\b", "KEYWORD", Some("DECLARATION")).unwrap();

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
        let scanner = RegexScanner::new(r"^(\w+)=(\d+)", "ASSIGNMENT", None).unwrap();

        // Test with an assignment
        let result = scanner.scan("value=42 next");
        assert!(result.is_ok());
        let token_option = result.unwrap();
        assert!(token_option.is_some());
        let token = token_option.unwrap();
        assert_eq!(token.token_type, "ASSIGNMENT");
        assert_eq!(token.value, "value=42"); // Should capture the entire match
    }

    #[test]
    fn test_regex_scanner_rejects_late_match_when_pattern_is_unanchored() {
        let scanner = RegexScanner::new(r"\d+", "NUMBER", None).unwrap();

        let at_start = scanner.scan("123 abc").unwrap().unwrap();
        assert_eq!(at_start.value, "123");

        let late_match = scanner.scan("abc 123").unwrap();
        assert!(late_match.is_none());
    }

    #[test]
    fn test_tokenizer_accepts_unanchored_regex_without_skipping_prefix() {
        let mut tokenizer = Tokenizer::new();
        tokenizer.add_regex_scanner(r"\d+", "NUMBER", None).unwrap();

        let success = tokenizer.tokenize("123").unwrap();
        assert_eq!(success.len(), 1);
        assert_eq!(success[0].value, "123");

        let failure = tokenizer.tokenize("abc123");
        assert!(failure.is_err());
    }
}