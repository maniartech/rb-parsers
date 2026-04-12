//! Fuzz target: feeds arbitrary UTF-8 bytes to a
//! [`rb_tokenizer::Tokenizer`] configured as a minimal JSON tokenizer.
//!
//! Run with:
//! ```sh
//! cargo +nightly fuzz run tokenizer
//! ```
#![no_main]

use libfuzzer_sys::fuzz_target;
use rb_tokenizer::{Tokenizer, TokenizerConfig};

fn make_tokenizer() -> Tokenizer {
    let config = TokenizerConfig {
        tokenize_whitespace: false,
        continue_on_error: true,
        error_tolerance_limit: 64,
        track_token_positions: true,
        ..Default::default()
    };
    let mut t = Tokenizer::with_config(config);
    // A small but representative token set — enough to exercise most scanner code paths.
    t.add_symbol_scanner("{", "LBRACE",   None);
    t.add_symbol_scanner("}", "RBRACE",   None);
    t.add_symbol_scanner("[", "LBRACKET", None);
    t.add_symbol_scanner("]", "RBRACKET", None);
    t.add_symbol_scanner(":", "COLON",    None);
    t.add_symbol_scanner(",", "COMMA",    None);
    t.add_regex_scanner(r#"^"([^"\\]|\\.)*""#, "STRING", None).unwrap();
    t.add_regex_scanner(r"^-?\d+(\.\d+)?([eE][-+]?\d+)?", "NUMBER", None).unwrap();
    t.add_regex_scanner(r"^(true|false|null)\b", "LITERAL", None).unwrap();
    t
}

// Build the tokenizer once for the lifetime of the fuzz process.
static TOKENIZER: std::sync::OnceLock<Tokenizer> = std::sync::OnceLock::new();

fuzz_target!(|data: &[u8]| {
    // Only test valid UTF-8; libFuzzer will still reach all byte patterns via
    // mutation — invalid UTF-8 is a precondition violation, not a bug.
    let Ok(input) = std::str::from_utf8(data) else { return };

    let tokenizer = TOKENIZER.get_or_init(make_tokenizer);
    // We do not care about success or failure; we only require *no panic*.
    let _ = tokenizer.tokenize(input);
});
