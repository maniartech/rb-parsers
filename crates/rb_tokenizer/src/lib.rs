#![warn(missing_docs)]
//! `rb_tokenizer` — a fast, zero-copy tokenizer framework.

/// Tokenizer error catalog — machine-readable codes for tokenizer diagnostics.
pub mod catalog;
/// Scanner combinators — composable `Scanner` implementations.
pub mod scanners;
/// Token source — buffer and slice-based token stream adapters.
pub mod token_source;
/// Token types — `Token`, `TokenId`, and related primitives.
pub mod tokens;
/// Tokenizer — the main `Tokenizer` and `BinaryTokenizer` entry points.
pub mod tokenizers;
/// Internal utility helpers.
pub mod utils;

// Re-export main types at crate root for easier access
pub use tokenizers::{BinaryToken, BinaryTokenizer, Tokenizer, TokenizerConfig};
pub use tokenizers::source_map::SourceMap;
pub use scanners::{
    BinaryScanner, CharClassScanner, ContextualClosureScanner, ContextualScanner,
    IndentationScanner, KeywordScanner, NumberLiteralScanner, OperatorScanner, ScanContext,
    WhitespaceScanner, WordBoundaryDef,
};
pub use token_source::{BufferedTokenSource, SliceTokenSource, TokenSource};

/// Placeholder addition function — present only to satisfy the default workspace template.
pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_utils() {
        let mut tokenizer = Tokenizer::new();
        tokenizer.add_regex_scanner(r"^\d+", "Number", None).unwrap();
        tokenizer.add_symbol_scanner("+", "Operator", Some("Plus"));

        let input = "123 + 456";
        let tokens = tokenizer.tokenize(input).unwrap();

        // Test pretty printing
        println!("\nPretty Print Example:");
        println!("{}", utils::pretty_print_tokens(&tokens));

        // Test token comparison
        let expected = vec![
            tokens::Token {
                token_type: "Number",
                token_sub_type: None,
                value: std::borrow::Cow::Borrowed("123"),
                span: tokens::SourceSpan::UNKNOWN,
            },
            tokens::Token {
                token_type: "Operator",
                token_sub_type: Some("Plus"),
                value: std::borrow::Cow::Borrowed("+"),
                span: tokens::SourceSpan::UNKNOWN,
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
