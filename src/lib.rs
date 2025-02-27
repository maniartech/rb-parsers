pub mod rules;
pub mod tokens;
pub mod tokenizers;

// Re-export main types at crate root for easier access
pub use tokenizers::{Tokenizer, TokenizerConfig};

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
