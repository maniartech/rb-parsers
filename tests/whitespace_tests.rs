use rb_tokenizer::{Tokenizer, TokenizerConfig};

fn get_whitespace_tokenizer() -> Tokenizer {
    let config = TokenizerConfig {
        tokenize_whitespace: true,
        continue_on_error: true,
        error_tolerance_limit: 5,
    };
    let mut tokenizer = Tokenizer::with_config(config);

    // Add simple rules
    tokenizer.add_regex_rule(r"^[a-zA-Z_][a-zA-Z0-9_]*", "Identifier", None);
    tokenizer.add_regex_rule(r"^\d+", "Number", None);
    
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
        
        // Expected tokens: Identifier, Whitespace, Whitespace, Identifier, Whitespace(newline), Number
        assert_eq!(result.len(), 6, "Unexpected number of tokens");
        
        // Verify first whitespace token (space)
        assert_eq!(result[1].token_type, "Whitespace");
        assert_eq!(result[1].value, " ");
        assert_eq!(result[1].token_sub_type, None);
        
        // Verify tab whitespace token
        assert_eq!(result[2].token_type, "Whitespace");
        assert_eq!(result[2].value, "\t");
        assert_eq!(result[2].token_sub_type, None);
        
        // Verify newline whitespace token
        assert_eq!(result[4].token_type, "Whitespace");
        assert_eq!(result[4].value, "\n");
        assert_eq!(result[4].token_sub_type, Some("Newline".to_string()));
        
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
