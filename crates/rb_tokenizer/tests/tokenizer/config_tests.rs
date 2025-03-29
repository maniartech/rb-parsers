use rb_tokenizer::{Tokenizer, TokenizerConfig};

#[cfg(test)]
mod config_tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let tokenizer = Tokenizer::new();
        let config = tokenizer.config();

        // Test default values
        assert!(!config.tokenize_whitespace, "Default tokenize_whitespace should be false");
        assert!(config.continue_on_error, "Default continue_on_error should be true");
        assert_eq!(config.error_tolerance_limit, 10, "Default error_tolerance_limit should be 10");
        assert!(config.track_token_positions, "Default track_token_positions should be true");
    }

    #[test]
    fn test_custom_config() {
        let custom_config = TokenizerConfig {
            tokenize_whitespace: true,
            continue_on_error: false,
            error_tolerance_limit: 5,
            track_token_positions: false,
        };

        let tokenizer = Tokenizer::with_config(custom_config);
        let config = tokenizer.config();

        assert!(config.tokenize_whitespace, "Custom tokenize_whitespace should be true");
        assert!(!config.continue_on_error, "Custom continue_on_error should be false");
        assert_eq!(config.error_tolerance_limit, 5, "Custom error_tolerance_limit should be 5");
        assert!(!config.track_token_positions, "Custom track_token_positions should be false");
    }

    #[test]
    fn test_config_setters() {
        let mut tokenizer = Tokenizer::new();

        // Use fluent API to configure tokenizer
        tokenizer
            .set_tokenize_whitespace(true)
            .set_continue_on_error(false)
            .set_error_tolerance_limit(20)
            .set_track_token_positions(false);

        let config = tokenizer.config();

        assert!(config.tokenize_whitespace);
        assert!(!config.continue_on_error);
        assert_eq!(config.error_tolerance_limit, 20);
        assert!(!config.track_token_positions);
    }

    #[test]
    fn test_with_options_method() {
        let mut tokenizer = Tokenizer::new();

        // Use the with_options method to set multiple options at once
        tokenizer.with_options(
            Some(true),   // tokenize_whitespace
            None,         // continue_on_error (unchanged)
            Some(15),     // error_tolerance_limit
            Some(false)   // track_token_positions
        );

        let config = tokenizer.config();

        assert!(config.tokenize_whitespace);
        assert!(config.continue_on_error, "Should remain unchanged");
        assert_eq!(config.error_tolerance_limit, 15);
        assert!(!config.track_token_positions);
    }

    #[test]
    fn test_config_cloning() {
        let original_config = TokenizerConfig {
            tokenize_whitespace: true,
            continue_on_error: false,
            error_tolerance_limit: 7,
            track_token_positions: false,
        };

        // Clone the config and verify it's equal
        let cloned_config = original_config.clone();

        assert_eq!(cloned_config.tokenize_whitespace, original_config.tokenize_whitespace);
        assert_eq!(cloned_config.continue_on_error, original_config.continue_on_error);
        assert_eq!(cloned_config.error_tolerance_limit, original_config.error_tolerance_limit);
        assert_eq!(cloned_config.track_token_positions, original_config.track_token_positions);
    }

    #[test]
    fn test_config_debug_output() {
        let config = TokenizerConfig {
            tokenize_whitespace: true,
            continue_on_error: false,
            error_tolerance_limit: 3,
            track_token_positions: true,
        };

        let debug_output = format!("{:?}", config);

        // Check debug output contains all fields
        assert!(debug_output.contains("tokenize_whitespace: true"));
        assert!(debug_output.contains("continue_on_error: false"));
        assert!(debug_output.contains("error_tolerance_limit: 3"));
        assert!(debug_output.contains("track_token_positions: true"));
    }
}