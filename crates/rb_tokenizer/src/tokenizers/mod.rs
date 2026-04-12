/// Binary tokenizer implementation.
pub mod binary_tokenizer;
/// Prebuilt newline-position index for O(log n) line/column lookup.
pub mod source_map;
/// Main `Tokenizer` and `TokenizerConfig` types.
pub mod tokenizer;

pub use binary_tokenizer::{BinaryToken, BinaryTokenizer};
pub use tokenizer::{Tokenizer, TokenizerConfig};
