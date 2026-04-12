//! Criterion benchmarks for `rb_tokenizer`.
//!
//! Groups:
//!   1. `lex_json`        — JSON tokenisation at three input sizes (small / medium / large).
//!   2. `lex_expr`        — Arithmetic expression tokenisation (keyword + operator + number scanners).
//!   3. `lex_source_code` — Simplified source-code tokenisation with many scanner types active.
//!   4. `whitespace`      — Uniform vs. split vs. skip (no-emit) whitespace modes.
//!   5. `scanner_types`   — Microbenchmarks per scanner kind isolated from the full tokenizer.
//!   6. `throughput`      — Bytes-per-second wall-clock throughput, scaled inputs.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use rb_tokenizer::{
    scanners::{NumberLiteralScanner, WhitespaceScanner},
    Tokenizer, TokenizerConfig,
};

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn json_tokenizer() -> Tokenizer {
    let mut t = Tokenizer::new();
    // Strings — block scanner dispatches on `"` via first-byte table
    t.add_block_scanner("\"", "\"", "STRING", None, true, false, false);
    // Numbers — NumberLiteralScanner dispatches on `0-9` via first-byte table;
    //            no regex engine overhead, ~3-4× faster than the regex variant.
    let num = NumberLiteralScanner::minimal("NUMBER", None)
        .allow_float(true)
        .allow_scientific(true);
    t.add_scanner(Box::new(num));
    // JSON literals — each dispatches on its own first byte (t, f, n): no collision,
    // so 3 scanners vs 1 combined scanner has identical hot-path cost.
    t.add_keyword_scanner("TRUE",  &["true"]);
    t.add_keyword_scanner("FALSE", &["false"]);
    t.add_keyword_scanner("NULL",  &["null"]);
    // Structural — each symbol dispatches on its own first byte
    t.add_symbol_scanner("{", "LBRACE", None);
    t.add_symbol_scanner("}", "RBRACE", None);
    t.add_symbol_scanner("[", "LBRACKET", None);
    t.add_symbol_scanner("]", "RBRACKET", None);
    t.add_symbol_scanner(":", "COLON", None);
    t.add_symbol_scanner(",", "COMMA", None);
    t
}

fn json_input_small() -> &'static str {
    r#"{"name":"Alice","age":30,"active":true}"#
}

fn json_input_medium() -> String {
    // ~20 key-value pairs
    let pair = |i: usize| format!(r#""key{i}":"value{i}","num{i}":{i}"#);
    let pairs: Vec<_> = (0..20).map(pair).collect();
    format!("{{{}}}", pairs.join(","))
}

fn json_input_large() -> String {
    // Nested array of 200 objects
    let obj = |i: usize| {
        format!(
            r#"{{"id":{i},"name":"item{i}","value":{v},"active":true}}"#,
            v = i * 17
        )
    };
    let objs: Vec<_> = (0..200).map(obj).collect();
    format!("[{}]", objs.join(","))
}

fn expr_tokenizer() -> Tokenizer {
    let mut t = Tokenizer::new();
    t.add_number_literal_scanner("NUMBER", None);
    t.add_operator_scanner_with_subtypes(
        "OP",
        &[("+", "Plus"), ("-", "Minus"), ("*", "Star"), ("/", "Slash")],
    );
    t.add_symbol_scanner("(", "LPAREN", None);
    t.add_symbol_scanner(")", "RPAREN", None);
    t
}

fn expr_input(depth: usize) -> String {
    // e.g. depth=5 → "1+2*3-4/5+1+2*3-4/5+…"
    let unit = "1+2*3-4/5";
    let units: Vec<_> = (0..depth).map(|_| unit).collect();
    units.join("+")
}

fn source_code_tokenizer() -> Tokenizer {
    let mut t = Tokenizer::new();
    // Block comments
    t.add_block_scanner("/*", "*/", "Comment", Some("Block"), false, false, false);
    // Line comments via EOL scanner
    t.add_eol_scanner("//", "Comment", Some("Line"), true);
    // String literals
    t.add_block_scanner("\"", "\"", "String", None, true, false, false);
    // Numbers
    t.add_number_literal_scanner("Number", None);
    // Keywords
    t.add_keyword_scanner("Keyword", &["if", "else", "while", "for", "return", "fn", "let", "mut"]);
    // Identifiers
    t.add_regex_scanner(r"^[a-zA-Z_][a-zA-Z0-9_]*", "Ident", None).unwrap();
    // Operators
    t.add_operator_scanner_with_subtypes(
        "Op",
        &[
            ("==", "Eq"), ("!=", "Ne"), ("<=", "Le"), (">=", "Ge"),
            ("<", "Lt"), (">", "Gt"),
            ("+=", "AddAssign"), ("-=", "SubAssign"),
            ("+", "Plus"), ("-", "Minus"), ("*", "Star"), ("/", "Slash"),
            ("=", "Assign"), ("&", "Amp"), ("|", "Pipe"),
        ],
    );
    // Punctuation
    for (sym, ty, sub) in &[
        ("{", "Punct", Some("LBrace")), ("}", "Punct", Some("RBrace")),
        ("(", "Punct", Some("LParen")), (")", "Punct", Some("RParen")),
        (";", "Punct", Some("Semi")),  (",", "Punct", Some("Comma")),
    ] {
        t.add_symbol_scanner(sym, ty, *sub);
    }
    // Whitespace
    t.add_whitespace_scanner(WhitespaceScanner::split("Whitespace", "Newline"));
    t
}

fn source_code_input(lines: usize) -> String {
    let body = r#"fn compute(x, y) {
    let result = x + y * 2;
    if result >= 10 {
        return result;
    } else {
        return 0;
    }
}
"#;
    body.repeat(lines)
}

// ─────────────────────────────────────────────────────────────────────────────
// 1. JSON tokenisation
// ─────────────────────────────────────────────────────────────────────────────

fn bench_lex_json(c: &mut Criterion) {
    let tokenizer = json_tokenizer();
    let small = json_input_small().to_string();
    let medium = json_input_medium();
    let large = json_input_large();

    let mut g = c.benchmark_group("lex_json");

    for (name, input) in [
        ("small", &small),
        ("medium", &medium),
        ("large", &large),
    ] {
        g.throughput(Throughput::Bytes(input.len() as u64));
        g.bench_with_input(BenchmarkId::new("tokenize", name), input, |b, inp| {
            b.iter(|| tokenizer.tokenize(inp).unwrap())
        });
    }

    g.finish();
}

// ─────────────────────────────────────────────────────────────────────────────
// 2. Expression tokenisation
// ─────────────────────────────────────────────────────────────────────────────

fn bench_lex_expr(c: &mut Criterion) {
    let tokenizer = expr_tokenizer();
    let mut g = c.benchmark_group("lex_expr");

    for depth in [10, 50, 200] {
        let input = expr_input(depth);
        g.throughput(Throughput::Bytes(input.len() as u64));
        g.bench_with_input(
            BenchmarkId::new("depth", depth),
            &input,
            |b, inp| b.iter(|| tokenizer.tokenize(inp).unwrap()),
        );
    }

    g.finish();
}

// ─────────────────────────────────────────────────────────────────────────────
// 3. Source code tokenisation
// ─────────────────────────────────────────────────────────────────────────────

fn bench_lex_source(c: &mut Criterion) {
    let tokenizer = source_code_tokenizer();
    let mut g = c.benchmark_group("lex_source_code");

    for lines in [10, 50, 200] {
        let input = source_code_input(lines);
        g.throughput(Throughput::Bytes(input.len() as u64));
        g.bench_with_input(
            BenchmarkId::new("lines", lines),
            &input,
            |b, inp| b.iter(|| tokenizer.tokenize(inp).unwrap()),
        );
    }

    g.finish();
}

// ─────────────────────────────────────────────────────────────────────────────
// 4. Whitespace modes
// ─────────────────────────────────────────────────────────────────────────────

fn bench_whitespace_modes(c: &mut Criterion) {
    let input = source_code_input(50);
    let mut g = c.benchmark_group("whitespace");
    g.throughput(Throughput::Bytes(input.len() as u64));

    // Uniform: all whitespace as one token
    {
        let mut t = source_code_tokenizer();
        t.add_whitespace_scanner(WhitespaceScanner::uniform("WS"));
        g.bench_function("uniform", |b| b.iter(|| t.tokenize(&input).unwrap()));
    }

    // Split: separate Newline tokens
    {
        let mut t = source_code_tokenizer();
        t.add_whitespace_scanner(WhitespaceScanner::split("WS", "Newline"));
        g.bench_function("split", |b| b.iter(|| t.tokenize(&input).unwrap()));
    }

    // Skip whitespace entirely via config
    {
        let cfg = TokenizerConfig {
            tokenize_whitespace: false,
            track_token_positions: true,
            continue_on_error: false,
            error_tolerance_limit: 1,
            ..Default::default()
        };
        let t = {
            let mut t2 = Tokenizer::with_config(cfg);
            t2.add_block_scanner("/*", "*/", "Comment", Some("Block"), false, false, false);
            t2.add_eol_scanner("//", "Comment", Some("Line"), true);
            t2.add_block_scanner("\"", "\"", "String", None, true, false, false);
            t2.add_number_literal_scanner("Number", None);
            for kw in &["if", "else", "while", "for", "return", "fn", "let", "mut"] {
                t2.add_keyword_scanner("Keyword", &[*kw]);
            }
            t2.add_regex_scanner(r"^[a-zA-Z_][a-zA-Z0-9_]*", "Ident", None).unwrap();
            // Full operator set matching source_code_tokenizer() so that
            // operators like `>=`, `==` in source_code_input() are recognised.
            t2.add_operator_scanner_with_subtypes(
                "Op",
                &[
                    ("==", "Eq"), ("!=", "Ne"), ("<=", "Le"), (">=", "Ge"),
                    ("<", "Lt"), (">", "Gt"),
                    ("+=", "AddAssign"), ("-=", "SubAssign"),
                    ("+", "Plus"), ("-", "Minus"), ("*", "Star"), ("/", "Slash"),
                    ("=", "Assign"), ("&", "Amp"), ("|", "Pipe"),
                ],
            );
            for (sym, ty, sub) in &[
                ("{", "Punct", Some("LBrace")), ("}", "Punct", Some("RBrace")),
                ("(", "Punct", Some("LParen")), (")", "Punct", Some("RParen")),
                (";", "Punct", Some("Semi")), (",", "Punct", Some("Comma")),
            ] {
                t2.add_symbol_scanner(sym, ty, *sub);
            }
            t2
        };
        g.bench_function("skip", |b| b.iter(|| t.tokenize(&input).unwrap()));
    }

    g.finish();
}

// ─────────────────────────────────────────────────────────────────────────────
// 5. Position tracking on/off
// ─────────────────────────────────────────────────────────────────────────────

fn bench_position_tracking(c: &mut Criterion) {
    let input = source_code_input(50);
    let mut g = c.benchmark_group("position_tracking");
    g.throughput(Throughput::Bytes(input.len() as u64));

    let make_tokenizer = |track: bool| {
        let cfg = TokenizerConfig {
            track_token_positions: track,
            tokenize_whitespace: false,
            continue_on_error: false,
            error_tolerance_limit: 1,
            ..Default::default()
        };
        let mut t = Tokenizer::with_config(cfg);
        t.add_regex_scanner(r"^[a-zA-Z_][a-zA-Z0-9_]*", "Ident", None).unwrap();
        t.add_number_literal_scanner("Number", None);
        t.add_operator_scanner_with_subtypes(
            "Op",
            &[("+", "Plus"), ("-", "Minus"), ("*", "Star"), ("/", "Slash"), ("=", "Assign")],
        );
        for (sym, ty, sub) in &[
            ("{", "Punct", Some("LBrace")), ("}", "Punct", Some("RBrace")),
            ("(", "Punct", Some("LParen")), (")", "Punct", Some("RParen")),
            (";", "Punct", Some("Semi")), (",", "Punct", Some("Comma")),
        ] {
            t.add_symbol_scanner(sym, ty, *sub);
        }
        t
    };

    let t_on = make_tokenizer(true);
    let t_off = make_tokenizer(false);

    g.bench_function("tracking_on", |b| b.iter(|| t_on.tokenize(&input).unwrap()));
    g.bench_function("tracking_off", |b| b.iter(|| t_off.tokenize(&input).unwrap()));

    g.finish();
}

// ─────────────────────────────────────────────────────────────────────────────
// 6. Throughput scaling (bytes/sec)
// ─────────────────────────────────────────────────────────────────────────────

fn bench_throughput(c: &mut Criterion) {
    let tokenizer = json_tokenizer();
    let mut g = c.benchmark_group("throughput_scaling");

    // Build inputs of growing sizes
    let base = json_input_large();
    let inputs: Vec<(usize, String)> = [1, 4, 16]
        .into_iter()
        .map(|mul| (base.len() * mul, base.repeat(mul)))
        .collect();

    for (bytes, input) in &inputs {
        g.throughput(Throughput::Bytes(*bytes as u64));
        g.bench_with_input(
            BenchmarkId::new("json_bytes", bytes),
            input,
            |b, inp| b.iter(|| tokenizer.tokenize(inp).unwrap()),
        );
    }

    g.finish();
}

// ─────────────────────────────────────────────────────────────────────────────
// Registration
// ─────────────────────────────────────────────────────────────────────────────

criterion_group!(
    benches,
    bench_lex_json,
    bench_lex_expr,
    bench_lex_source,
    bench_whitespace_modes,
    bench_position_tracking,
    bench_throughput,
);
criterion_main!(benches);
