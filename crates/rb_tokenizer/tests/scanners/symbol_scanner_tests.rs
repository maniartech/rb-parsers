use rb_tokenizer::{
    scanners::symbol_scanner::SymbolScanner,
    scanners::scanner::Scanner,
};

#[cfg(test)]
mod symbol_scanner_tests {
    use super::*;

    #[test]
    fn test_basic_symbol_scanner() {
        let scanner = SymbolScanner::new("if", "KEYWORD", None);

        // Test matching input
        let result = scanner.scan("if (x > 0) {", 0, 0);
        assert!(result.is_ok());
        let token_option = result.unwrap();
        assert!(token_option.is_some());
        let token = token_option.unwrap();
        assert_eq!(token.token_type, "KEYWORD");
        assert_eq!(token.value, "if");
        assert_eq!(token.token_sub_type, None);

        // Test non-matching input
        let result = scanner.scan("different text", 0, 0);
        assert!(result.is_ok());
        let token_option = result.unwrap();
        assert!(token_option.is_none());

        // Test substring match (should not match)
        let result = scanner.scan("iffy", 0, 0);
        assert!(result.is_ok());
        let token_option = result.unwrap();
        assert!(token_option.is_none());
    }

    #[test]
    fn test_symbol_scanner_with_subtype() {
        let scanner = SymbolScanner::new("for", "KEYWORD", Some("LOOP"));

        // Test with matching input
        let result = scanner.scan("for (i = 0; i < 10; i++) {", 0, 0);
        assert!(result.is_ok());
        let token_option = result.unwrap();
        assert!(token_option.is_some());
        let token = token_option.unwrap();
        assert_eq!(token.token_type, "KEYWORD");
        assert_eq!(token.value, "for");
        assert_eq!(token.token_sub_type, Some("LOOP".to_string()));
    }

    #[test]
    fn test_symbol_scanner_position_tracking() {
        let scanner = SymbolScanner::new("+", "OPERATOR", Some("ADDITION"));

        // Test with specific line and column
        let result = scanner.scan("+ 5", 3, 7);
        assert!(result.is_ok());
        let token_option = result.unwrap();
        assert!(token_option.is_some());
        let token = token_option.unwrap();
        assert_eq!(token.token_type, "OPERATOR");
        assert_eq!(token.value, "+");
        assert_eq!(token.line, 3);
        assert_eq!(token.column, 7);
    }

    #[test]
    fn test_symbol_scanner_with_empty_symbol() {
        let scanner = SymbolScanner::new("", "EMPTY", None);

        // Test with any input (should not match since empty symbol is invalid)
        let result = scanner.scan("test", 0, 0);
        assert!(result.is_ok());
        let token_option = result.unwrap();
        assert!(token_option.is_none());
    }

    #[test]
    fn test_symbol_scanner_with_special_characters() {
        // Test with various special characters
        let symbols = vec![
            ("==", "OPERATOR", Some("EQUALITY")),
            ("!=", "OPERATOR", Some("INEQUALITY")),
            ("&&", "OPERATOR", Some("LOGICAL_AND")),
            ("||", "OPERATOR", Some("LOGICAL_OR")),
            ("=>", "ARROW", None),
            ("{", "BRACE", Some("OPEN")),
            ("}", "BRACE", Some("CLOSE")),
        ];

        for (symbol, token_type, token_sub_type) in symbols {
            let scanner = SymbolScanner::new(symbol, token_type, token_sub_type);

            // Create input with the symbol
            let input = format!("{} rest", symbol);
            let result = scanner.scan(&input, 0, 0);
            assert!(result.is_ok());
            let token_option = result.unwrap();
            assert!(token_option.is_some());
            let token = token_option.unwrap();
            assert_eq!(token.token_type, token_type);
            assert_eq!(token.value, symbol);
            if let Some(sub_type) = token_sub_type {
                assert_eq!(token.token_sub_type, Some(sub_type.to_string()));
            } else {
                assert_eq!(token.token_sub_type, None);
            }
        }
    }

    #[test]
    fn test_symbol_scanner_case_sensitivity() {
        let scanner = SymbolScanner::new("SELECT", "SQL", None);

        // Test with exact case
        let result = scanner.scan("SELECT * FROM table", 0, 0);
        assert!(result.is_ok());
        let token_option = result.unwrap();
        assert!(token_option.is_some());

        // Test with different case (should not match)
        let result = scanner.scan("select * FROM table", 0, 0);
        assert!(result.is_ok());
        let token_option = result.unwrap();
        assert!(token_option.is_none());
    }
}