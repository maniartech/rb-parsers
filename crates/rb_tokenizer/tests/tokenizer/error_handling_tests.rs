use rb_tokenizer::{Tokenizer, TokenizerConfig};

#[cfg(test)]
mod error_handling_tests {
    use super::*;

    // Helper function to create a tokenizer with error recovery options
    fn create_test_tokenizer(continue_on_error: bool, error_limit: usize) -> Tokenizer {
        let config = TokenizerConfig {
            tokenize_whitespace: true,
            continue_on_error,
            error_tolerance_limit: error_limit,
            track_token_positions: true,
        };
        
        let mut tokenizer = Tokenizer::with_config(config);
        
        // Add a simple block scanner that requires closing tag
        tokenizer.add_block_scanner("<<", ">>", "BLOCK", None, false, false, true);
        
        tokenizer
    }

    #[test]
    fn test_strict_mode_fails_on_first_error() {
        // Create tokenizer with strict mode (continue_on_error = false)
        let tokenizer = create_test_tokenizer(false, 1);
        
        // Input with an unclosed block - should fail
        let input = "Some text << unclosed block";
        
        let result = tokenizer.tokenize(input);
        assert!(result.is_err(), "Strict mode should fail on unclosed block");
        
        if let Err(e) = result {
            assert!(e.message.contains("unclosed") || e.message.contains("Unclosed") || 
                   e.message.contains("mismatch") || e.message.contains("missing"),
                   "Error should indicate unclosed block");
        }
    }
    
    #[test]
    fn test_tolerant_mode_continues_after_errors() {
        // Create tokenizer with tolerant mode (continue_on_error = true)
        let mut tokenizer = create_test_tokenizer(true, 5);
        
        // Input with multiple problems
        let input = "Text <<block1>> more <<unclosed1 and <<unclosed2 and some <<good>> block";
        
        let result = tokenizer.tokenize(input);
        assert!(result.is_ok(), "Tolerant mode should not fail on unclosed blocks");
        
        // Check that we stored errors
        let errors = tokenizer.last_errors();
        assert!(errors.is_some(), "Should have stored errors");
        
        if let Some(err_vec) = errors {
            assert!(err_vec.len() >= 2, "Should have at least 2 errors for unclosed blocks");
            
            // Print errors for clarity
            for (i, e) in err_vec.iter().enumerate() {
                println!("Error {}: {:?}", i, e);
            }
        }
        
        // Verify we got some tokens despite errors
        if let Ok(tokens) = result {
            // We should have tokens for the text and complete blocks
            assert!(!tokens.is_empty(), "Should have tokens despite errors");
            
            // We should have at least one successful BLOCK token
            let block_tokens = tokens.iter()
                .filter(|t| t.token_type == "BLOCK")
                .count();
                
            assert!(block_tokens > 0, "Should have at least one successful BLOCK token");
            
            // Print tokens for clarity
            println!("Found {} tokens:", tokens.len());
            for (i, t) in tokens.iter().enumerate() {
                println!("Token {}: {:?}", i, t);
            }
        }
    }
    
    #[test]
    fn test_error_limit_behavior() {
        // Create tokenizer with a low error limit
        let mut tokenizer = create_test_tokenizer(true, 2);
        
        // Input with more errors than the limit
        let input = "<<unclosed1 text <<unclosed2 more <<unclosed3";
        
        let result = tokenizer.tokenize(input);
        assert!(result.is_err(), "Should fail when error limit is exceeded");
        
        if let Err(e) = result {
            assert!(e.message.contains("limit") || e.message.contains("tolerance") || 
                    e.message.contains("too many"),
                   "Error should indicate exceeding error limit");
            println!("Expected error when limit exceeded: {}", e);
        }
        
        // Verify the errors were still stored
        let errors = tokenizer.last_errors();
        assert!(errors.is_some(), "Should have stored errors");
        
        if let Some(err_vec) = errors {
            assert!(err_vec.len() >= 2, "Should have at least 2 errors stored");
            println!("Found {} errors:", err_vec.len());
            for (i, e) in err_vec.iter().enumerate() {
                println!("Error {}: {:?}", i, e);
            }
        }
    }
    
    #[test]
    fn test_error_reset_between_tokenizations() {
        let mut tokenizer = create_test_tokenizer(true, 10);
        
        // First input with errors
        let input1 = "Some text <<unclosed";
        let result1 = tokenizer.tokenize(input1);
        assert!(result1.is_ok(), "Tolerant mode should not fail on unclosed block");
        
        // Verify we have errors from first tokenization
        let errors1 = tokenizer.last_errors();
        assert!(errors1.is_some(), "Should have errors after first tokenization");
        assert!(!errors1.unwrap().is_empty(), "Should have at least one error");
        
        // Second input with no errors
        let input2 = "Correct <<block>>";
        let result2 = tokenizer.tokenize(input2);
        assert!(result2.is_ok(), "Should succeed with correct input");
        
        // Verify there are no errors from second tokenization
        let errors2 = tokenizer.last_errors();
        assert!(errors2.is_some(), "Last errors should be an empty vec, not None");
        assert!(errors2.unwrap().is_empty(), "Should have no errors after correct input");
    }
    
    #[test]
    fn test_error_positions() {
        let mut tokenizer = create_test_tokenizer(true, 5);
        
        // Input with error at specific position
        let input = "Line 1\nLine 2\nLine 3 <<unclosed\nLine 4";
        
        let _ = tokenizer.tokenize(input);
        let errors = tokenizer.last_errors().unwrap();
        
        assert!(!errors.is_empty(), "Should have errors");
        
        // Error should be on line 3
        let error = &errors[0];
        assert_eq!(error.line, 3, "Error should be on line 3");
        
        // Column depends on implementation, but should be somewhere after the opening tag
        assert!(error.column >= "Line 3 ".len(), 
                "Error column should be at least after 'Line 3 '");
    }
}