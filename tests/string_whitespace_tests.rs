use rb_tokenizer::{Tokenizer, TokenizerConfig};

fn get_string_tokenizer() -> Tokenizer {
    let config = TokenizerConfig {
        tokenize_whitespace: true,
        continue_on_error: true,
        error_tolerance_limit: 5,
        track_token_positions: true,
    };
    let mut tokenizer = Tokenizer::with_config(config);

    // Add string rules with different quote styles
    tokenizer.add_regex_rule(r#"^"([^"\\]|\\.)*""#, "String", Some("DoubleQuoted"));
    tokenizer.add_regex_rule(r#"^'([^'\\]|\\.)*'"#, "String", Some("SingleQuoted"));
    // Updated backtick string rule to handle raw string escapes
    tokenizer.add_regex_rule(r#"^`([^`])*`"#, "String", Some("Backtick"));
    
    // Add other rules
    tokenizer.add_regex_rule(r"^[a-zA-Z_][a-zA-Z0-9_]*", "Identifier", None);
    tokenizer.add_symbol_rule("=", "Operator", Some("Assignment"));
    tokenizer.add_symbol_rule(";", "Semicolon", None);
    
    tokenizer
}

#[cfg(test)]
mod whitespace_in_strings_tests {
    use super::*;
    
    #[test]
    fn test_whitespace_in_strings() {
        let tokenizer = get_string_tokenizer();
        
        // Test with double-quoted string containing whitespace
        let input = r#"var = "hello world with spaces";"#;
        let result = tokenizer.tokenize(input).expect("Tokenization failed");
        
        // Expected tokens: Identifier, Whitespace, Operator, Whitespace, String, Semicolon
        assert_eq!(result.len(), 6, "Unexpected number of tokens");
        
        // Verify the string token preserves internal whitespace
        assert_eq!(result[4].token_type, "String");
        assert_eq!(result[4].value, r#""hello world with spaces""#);
        
        println!("String whitespace preservation: {:?}", result);
    }
    
    #[test]
    fn test_mixed_whitespace_types() {
        let tokenizer = get_string_tokenizer();
        
        // Test with various types of whitespace in strings
        let input = "var = \"hello\tworld\nwith\rdifferent\twhitespace\";";
        let result = tokenizer.tokenize(input).expect("Tokenization failed");
        
        // Verify the string token preserves all types of whitespace
        assert_eq!(result[4].token_type, "String");
        assert_eq!(result[4].value, "\"hello\tworld\nwith\rdifferent\twhitespace\"");
        
        println!("Mixed whitespace types: {:?}", result);
    }
    
    #[test]
    fn test_multiple_string_types() {
        let tokenizer = get_string_tokenizer();
        
        // Test with different quote styles
        let input = r#"double = "spaces here"; single = 'more spaces'; backtick = `even\\tmore`;"#;
        let result = tokenizer.tokenize(input).expect("Tokenization failed");
        
        // Find string tokens by their values
        let double_quoted = result.iter().find(|t| t.value.starts_with('"')).expect("Double quoted string not found");
        let single_quoted = result.iter().find(|t| t.value.starts_with('\'')).expect("Single quoted string not found");
        let backtick = result.iter().find(|t| t.value.starts_with('`')).expect("Backtick string not found");
        
        assert_eq!(double_quoted.value, r#""spaces here""#);
        assert_eq!(single_quoted.value, "'more spaces'");
        // Raw strings preserve backslashes exactly as they appear in the input
        assert_eq!(backtick.value, r#"`even\\\\\\\\tmore`"#);
        
        println!("Multiple string types: {:?}", result);
    }
    
    #[test]
    fn test_whitespace_tokens_outside_strings() {
        let tokenizer = get_string_tokenizer();
        
        // Test that whitespace outside strings is properly tokenized
        let input = "var = \t \"value\";";
        let result = tokenizer.tokenize(input).expect("Tokenization failed");
        
        // Check that whitespace outside strings is tokenized separately
        assert_eq!(result[1].token_type, "Whitespace");
        assert_eq!(result[3].token_type, "Whitespace");
        // The whitespace token should exactly match the whitespace in the input
        assert_eq!(result[3].value, "\t ");
        
        // But whitespace inside strings remains part of the string token
        let string_token = result.iter().find(|t| t.token_type == "String").expect("String token not found");
        assert_eq!(string_token.value, "\"value\"");
        
        println!("Whitespace tokens outside strings: {:?}", result);
    }
}