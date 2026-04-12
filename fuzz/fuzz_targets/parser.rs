//! Fuzz target: feeds arbitrary UTF-8 bytes through the full tokenizer →
//! parser pipeline, exercising the entire parse engine on random input.
//!
//! Run with:
//! ```sh
//! cargo +nightly fuzz run parser
//! ```
#![no_main]

use libfuzzer_sys::fuzz_target;
use rb_common::DiagnosticsContext;
use rb_parser::CompiledParser;
use rb_tokenizer::{Tokenizer, TokenizerConfig};

// ── JSON tokenizer ────────────────────────────────────────────────────────────

fn make_tokenizer() -> Tokenizer {
    let config = TokenizerConfig {
        tokenize_whitespace: false,
        continue_on_error: true,
        error_tolerance_limit: 128,
        track_token_positions: true,
        ..Default::default()
    };
    let mut t = Tokenizer::with_config(config);
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

// ── JSON parser ───────────────────────────────────────────────────────────────

use rb_parser::prelude::*;

// Rule IDs for the JSON grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum JsonRule { Value, Object, Array, Pair, Primitive }
impl RuleId for JsonRule {}

fn make_parser() -> CompiledParser {
    let profile = ResolvedProfile::simple("fuzz_json");
    grammar::<JsonRule>()
        .rule(JsonRule::Primitive, node(SyntaxKind::new("primitive"), one_of!(
            tok("STRING"), tok("NUMBER"), tok("LITERAL")
        )))
        .rule(JsonRule::Pair, node(SyntaxKind::new("pair"), seq!(
            field("key",   tok("STRING")),
            tok("COLON"),
            field("value", ref_(JsonRule::Value))
        )))
        .rule(JsonRule::Object, node(SyntaxKind::new("object"),
            between(tok("LBRACE"), list(ref_(JsonRule::Pair), tok("COMMA")), tok("RBRACE"))
        ))
        .rule(JsonRule::Array, node(SyntaxKind::new("array"),
            between(tok("LBRACKET"), list(ref_(JsonRule::Value), tok("COMMA")), tok("RBRACKET"))
        ))
        .rule(JsonRule::Value, node(SyntaxKind::new("value"), one_of!(
            ref_(JsonRule::Object),
            ref_(JsonRule::Array),
            ref_(JsonRule::Primitive)
        )))
        .start(JsonRule::Value)
        .compile(&profile)
        .expect("JSON grammar must compile")
}

// Build both once per process.
static TOKENIZER: std::sync::OnceLock<Tokenizer>       = std::sync::OnceLock::new();
static PARSER:    std::sync::OnceLock<CompiledParser>  = std::sync::OnceLock::new();

fuzz_target!(|data: &[u8]| {
    let Ok(input) = std::str::from_utf8(data) else { return };

    let tokenizer = TOKENIZER.get_or_init(make_tokenizer);
    let parser    = PARSER.get_or_init(make_parser);

    // Tokenise — errors are fine; we need tokens for the parser.
    let Ok(tokens) = tokenizer.tokenize(input) else { return };

    // Parse — must not panic regardless of token stream.
    let mut ctx = DiagnosticsContext::new();
    let _ = parser.parse_tree(&tokens, &mut ctx);
});
