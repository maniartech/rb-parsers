use rb_tokenizer::{Tokenizer, TokenizerConfig};

fn get_whitespace_tokenizer() -> Tokenizer {
    let config = TokenizerConfig {
        tokenize_whitespace: true,
        continue_on_error: true,
        error_tolerance_limit: 5,
        track_token_positions: true,
    };
    let mut tokenizer = Tokenizer::with_config(config);

    // Add simple scanners
    tokenizer.add_regex_scanner(r"^[a-zA-Z_][a-zA-Z0-9_]*", "Identifier", None);
    tokenizer.add_regex_scanner(r"^\d+", "Number", None);
    
    tokenizer
}

#[cfg(test)]
mod whitespace_tests {
    use super::*;
    
    #[test]
    fn test_whitespace_tokenization() {
        let tokenizer = get_whitespace_tokenizer();
        
        // Test string with various whitespace characters
        let input = "hello \t world\n123";
        let result = tokenizer.tokenize(input).expect("Tokenization failed");
        
        // Our tokenizer now combines consecutive whitespace characters into a single token
        // Expected tokens: Identifier, Whitespace(space+tab+space), Identifier, Whitespace(newline), Number
        assert_eq!(result.len(), 5, "Unexpected number of tokens");
        
        // Verify combined whitespace token
        assert_eq!(result[1].token_type, "Whitespace");
        assert!(result[1].value.contains(' '), "Whitespace should contain space");
        assert!(result[1].value.contains('\t'), "Whitespace should contain tab");
        assert_eq!(result[1].token_sub_type, None);
        
        // Verify newline whitespace token
        assert_eq!(result[3].token_type, "Whitespace");
        assert_eq!(result[3].value, "\n");
        assert_eq!(result[3].token_sub_type, Some("Newline".to_string()));
        
        println!("Whitespace tokens: {:?}", result);
    }
    
    #[test]
    fn test_whitespace_line_column_tracking() {
        let tokenizer = get_whitespace_tokenizer();
        
        // Test string with newline to check line counting
        let input = "hello\nworld";
        let result = tokenizer.tokenize(input).expect("Tokenization failed");
        
        // Expected tokens: Identifier, Whitespace(newline), Identifier
        assert_eq!(result.len(), 3, "Unexpected number of tokens");
        
        // Check line and column of first token
        assert_eq!(result[0].line, 1);
        assert_eq!(result[0].column, 1);
        
        // Check line and column of newline token
        assert_eq!(result[1].line, 1);
        assert_eq!(result[1].column, 6);
        
        // Check line and column of second identifier (should be on line 2)
        assert_eq!(result[2].line, 2);
        assert_eq!(result[2].column, 1);
        
        println!("Line and column tracking: {:?}", result);
    }
}
