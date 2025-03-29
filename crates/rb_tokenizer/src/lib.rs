pub mod scanners;
pub mod tokens;
pub mod tokenizers;
pub mod utils;

// Re-export main types at crate root for easier access
pub use tokenizers::{Tokenizer, TokenizerConfig};

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_utils() {
        let mut tokenizer = Tokenizer::new();
        tokenizer.add_regex_scanner(r"^\d+", "Number", None);
        tokenizer.add_symbol_scanner("+", "Operator", Some("Plus"));

        let input = "123 + 456";
        let tokens = tokenizer.tokenize(input).unwrap();

        // Test pretty printing
        println!("\nPretty Print Example:");
        println!("{}", utils::pretty_print_tokens(&tokens));

        // Test token comparison
        let expected = vec![
            tokens::Token {
                token_type: "Number".to_string(),
                token_sub_type: None,
                value: "123".to_string(),
                line: 1,
                column: 1,
            },
            tokens::Token {
                token_type: "Operator".to_string(),
                token_sub_type: Some("Plus".to_string()),
                value: "+".to_string(),
                line: 1,
                column: 5,
            },
        ];

        println!("\nComparison Example:");
        println!("{}", utils::compare_tokens(&expected, &tokens));

        // Test position visualization
        println!("\nPosition Visualization Example:");
        println!("{}", utils::visualize_token_positions(input, &tokens));

        // Test token analysis
        println!("\nToken Analysis Example:");
        println!("{}", utils::analyze_tokens(&tokens));
    }

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
