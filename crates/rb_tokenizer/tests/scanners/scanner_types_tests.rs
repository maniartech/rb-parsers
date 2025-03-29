use rb_tokenizer::{
    scanners::scanner_types::{Position, ScanResult},
    tokens::{Token, TokenizationError},
};

#[cfg(test)]
mod scanner_types_tests {
    use super::*;

    #[test]
    fn test_position() {
        let pos = Position::new(5, 10);

        assert_eq!(pos.line, 5);
        assert_eq!(pos.column, 10);

        // Test helper methods if available
        let advanced_pos = pos.advance_column(3);
        assert_eq!(advanced_pos.line, 5);
        assert_eq!(advanced_pos.column, 13);

        let new_line_pos = pos.advance_line();
        assert_eq!(new_line_pos.line, 6);
        assert_eq!(new_line_pos.column, 0); // Column resets to 0 for a new line
    }

    #[test]
    fn test_scan_result() {
        // Test Ok variant with Some token
        let token = Token {
            token_type: "TEST".to_string(),
            token_sub_type: None,
            value: "test_value".to_string(),
            line: 1,
            column: 2,
        };

        let ok_some_result: ScanResult = Ok(Some(token.clone()));

        if let Ok(Some(t)) = &ok_some_result {
            assert_eq!(t.token_type, "TEST");
            assert_eq!(t.value, "test_value");
            assert_eq!(t.line, 1);
            assert_eq!(t.column, 2);
        } else {
            panic!("Expected Ok(Some(token))");
        }

        // Test Ok variant with None
        let ok_none_result: ScanResult = Ok(None);

        if let Ok(None) = &ok_none_result {
            // This is the expected case
        } else {
            panic!("Expected Ok(None)");
        }

        // Test Err variant
        let error = TokenizationError::new("Test error", 3, 4);
        let err_result: ScanResult = Err(error);

        if let Err(e) = &err_result {
            assert_eq!(e.message, "Test error");
            assert_eq!(e.line, 3);
            assert_eq!(e.column, 4);
        } else {
            panic!("Expected Err(error)");
        }
    }

    #[test]
    fn test_scan_result_combinators() {
        // Create tokens and errors
        let token1 = Token {
            token_type: "TYPE1".to_string(),
            token_sub_type: None,
            value: "value1".to_string(),
            line: 1,
            column: 0,
        };

        let token2 = Token {
            token_type: "TYPE2".to_string(),
            token_sub_type: None,
            value: "value2".to_string(),
            line: 2,
            column: 5,
        };

        let error = TokenizationError::new("Test error", 3, 10);

        // Test and_then (if available)
        let result1: ScanResult = Ok(Some(token1.clone()));
        let result2: ScanResult = Ok(Some(token2.clone()));
        let err_result: ScanResult = Err(error.clone());

        // Test Result's map method
        let mapped_result = result1.map(|opt_token| {
            opt_token.map(|mut token| {
                token.token_sub_type = Some("MAPPED".to_string());
                token
            })
        });

        if let Ok(Some(t)) = mapped_result {
            assert_eq!(t.token_sub_type, Some("MAPPED".to_string()));
        } else {
            panic!("Expected Ok(Some(token)) after mapping");
        }

        // Test Result's unwrap_or
        let unwrap_or_none = Ok::<Option<Token>, TokenizationError>(None)
            .unwrap_or(None);
        assert!(unwrap_or_none.is_none());

        let unwrap_or_err = err_result.clone()
            .unwrap_or(Some(token2.clone()));
        assert!(unwrap_or_err.is_some());
        assert_eq!(unwrap_or_err.unwrap().token_type, "TYPE2");

        // Test Result's or_else (if chaining is important)
        let or_else_result = err_result
            .or_else(|_| Ok(Some(token1.clone())));

        if let Ok(Some(t)) = or_else_result {
            assert_eq!(t.token_type, "TYPE1");
        } else {
            panic!("Expected Ok(Some(token)) after or_else");
        }
    }
}