//! Criterion benchmarks for `rb_parser`.
//!
//! Groups:
//!   1. `parse_json`           — JSON grammar: small / medium / large token streams.
//!   2. `parse_expr_pratt`     — Pratt expression: flat chains and deeply nested via parens.
//!   3. `parse_events_vs_tree` — `parse_events` vs `parse_tree` for the same input.
//!   4. `parse_error_recovery` — Throughput when the token stream contains errors.
//!   5. `parse_deeply_nested`  — Stress test: deeply recursive grammars.
//!   6. `parse_throughput`     — Bytes-per-second scaling across input magnitudes.
//!   7. `vs_serde_json`        — End-to-end: rb pipeline vs serde_json on identical inputs.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use rb_common::diagnostics::DiagnosticsContext;
use rb_parser::prelude::*;
use rb_tokenizer::{tokens::{SourceSpan, Token}, Tokenizer};
use std::borrow::Cow;

// ─────────────────────────────────────────────────────────────────────────────
// Token helpers
// ─────────────────────────────────────────────────────────────────────────────

fn t(ty: &'static str, val: &'static str) -> Token<'static> {
    Token {
        token_type: ty,
        token_sub_type: None,
        value: Cow::Borrowed(val),
        span: SourceSpan::UNKNOWN,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Grammar factories
// ─────────────────────────────────────────────────────────────────────────────

// ── JSON ─────────────────────────────────────────────────────────────────────

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
enum JsonRule { Value, Object, Array, Pair, Primitive }
impl RuleId for JsonRule {}

fn build_json_parser() -> rb_parser::CompiledParser {
    let profile = ResolvedProfile::simple("json");
    grammar::<JsonRule>()
        .rule(
            JsonRule::Primitive,
            node(SyntaxKind::new("primitive"), one_of!(
                tok("STRING"), tok("NUMBER"), tok("TRUE"), tok("FALSE"), tok("NULL")
            )),
        )
        .rule(
            JsonRule::Pair,
            node(SyntaxKind::new("pair"), seq!(
                field("key", tok("STRING")),
                tok("COLON"),
                field("value", ref_(JsonRule::Value))
            )),
        )
        .rule(
            JsonRule::Object,
            node(SyntaxKind::new("object"),
                between(tok("LBRACE"), list(ref_(JsonRule::Pair), tok("COMMA")), tok("RBRACE"))),
        )
        .rule(
            JsonRule::Array,
            node(SyntaxKind::new("array"),
                between(tok("LBRACKET"), list(ref_(JsonRule::Value), tok("COMMA")), tok("RBRACKET"))),
        )
        .rule(
            JsonRule::Value,
            node(SyntaxKind::new("value"), one_of!(
                ref_(JsonRule::Object), ref_(JsonRule::Array), ref_(JsonRule::Primitive)
            )),
        )
        .start(JsonRule::Value)
        .compile(&profile)
        .unwrap()
}

/// `[1, 2, 3]`
fn json_tokens_small() -> Vec<Token<'static>> {
    vec![
        t("LBRACKET", "["),
        t("NUMBER", "1"), t("COMMA", ","),
        t("NUMBER", "2"), t("COMMA", ","),
        t("NUMBER", "3"),
        t("RBRACKET", "]"),
    ]
}

/// `{"k0": 0, "k1": 1, …, "k49": 49}`
fn json_tokens_medium() -> Vec<Token<'static>> {
    let mut v = vec![t("LBRACE", "{")];
    for i in 0usize..50 {
        if i > 0 { v.push(t("COMMA", ",")); }
        v.push(t("STRING", "\"k\""));
        v.push(t("COLON", ":"));
        v.push(t("NUMBER", "0"));
    }
    v.push(t("RBRACE", "}"));
    v
}

/// Nested array 3 levels deep, 100 leaf numbers each
fn json_tokens_large() -> Vec<Token<'static>> {
    // [ [ [ 0, 1, …, 99 ], [ 0..99 ] ], [ [ … ], [ … ] ] ]
    let leaf_array = || {
        let mut v = vec![t("LBRACKET", "[")];
        for i in 0usize..100 {
            if i > 0 { v.push(t("COMMA", ",")); }
            v.push(t("NUMBER", "0"));
        }
        v.push(t("RBRACKET", "]"));
        v
    };
    let mut outer = vec![t("LBRACKET", "[")];
    for j in 0..4 {
        if j > 0 { outer.push(t("COMMA", ",")); }
        outer.push(t("LBRACKET", "["));
        for k in 0..2 {
            if k > 0 { outer.push(t("COMMA", ",")); }
            outer.extend(leaf_array());
        }
        outer.push(t("RBRACKET", "]"));
    }
    outer.push(t("RBRACKET", "]"));
    outer
}

// ── Pratt expression ─────────────────────────────────────────────────────────

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
enum ExprRule { Expr, Atom }
impl RuleId for ExprRule {}

fn build_expr_parser() -> rb_parser::CompiledParser {
    let profile = ResolvedProfile::simple("expr");
    let pratt_spec = pratt::<ExprRule>(ref_(ExprRule::Atom))
        .prefix("-", 5, SyntaxKind::new("neg"))
        .infix_left("+", 1, SyntaxKind::new("add"))
        .infix_left("-", 1, SyntaxKind::new("sub"))
        .infix_left("*", 3, SyntaxKind::new("mul"))
        .infix_left("/", 3, SyntaxKind::new("div"))
        .postfix("!", 7, SyntaxKind::new("fact"))
        .finish();
    grammar::<ExprRule>()
        .rule(ExprRule::Atom, node(SyntaxKind::new("atom"), tok("NUM")))
        .rule(ExprRule::Expr, node(SyntaxKind::new("expr"), pratt_spec))
        .start(ExprRule::Expr)
        .compile(&profile)
        .unwrap()
}

/// `1 + 2 * 3 - 4 / 5`  repeated `n` times joined with `+`
fn expr_tokens_chain(n: usize) -> Vec<Token<'static>> {
    let unit: Vec<Token<'static>> = vec![
        t("NUM", "1"), t("+", "+"), t("NUM", "2"), t("*", "*"),
        t("NUM", "3"), t("-", "-"), t("NUM", "4"), t("/", "/"), t("NUM", "5"),
    ];
    let mut v = Vec::with_capacity(n * (unit.len() + 1));
    for i in 0..n {
        if i > 0 { v.push(t("+", "+")); }
        v.extend(unit.iter().cloned());
    }
    v
}

// ─────────────────────────────────────────────────────────────────────────────
// 1. parse_json
// ─────────────────────────────────────────────────────────────────────────────

fn bench_parse_json(c: &mut Criterion) {
    let parser = build_json_parser();
    let small = json_tokens_small();
    let medium = json_tokens_medium();
    let large = json_tokens_large();

    let mut g = c.benchmark_group("parse_json");

    for (name, tokens) in [("small", &small), ("medium", &medium), ("large", &large)] {
        let byte_estimate = tokens.iter().map(|t| t.value.len()).sum::<usize>() as u64;
        g.throughput(Throughput::Elements(tokens.len() as u64));
        g.bench_with_input(BenchmarkId::new("parse_tree", name), tokens, |b, toks| {
            b.iter(|| {
                let mut ctx = DiagnosticsContext::new();
                parser.parse_tree(toks, &mut ctx)
            })
        });
        let _ = byte_estimate;
    }

    g.finish();
}

// ─────────────────────────────────────────────────────────────────────────────
// 2. parse_expr_pratt
// ─────────────────────────────────────────────────────────────────────────────

fn bench_parse_expr_pratt(c: &mut Criterion) {
    let parser = build_expr_parser();
    let mut g = c.benchmark_group("parse_expr_pratt");

    for n in [5, 20, 100] {
        let tokens = expr_tokens_chain(n);
        g.throughput(Throughput::Elements(tokens.len() as u64));
        g.bench_with_input(BenchmarkId::new("chain", n), &tokens, |b, toks| {
            b.iter(|| {
                let mut ctx = DiagnosticsContext::new();
                parser.parse_tree(toks, &mut ctx)
            })
        });
    }

    g.finish();
}

// ─────────────────────────────────────────────────────────────────────────────
// 3. parse_events vs parse_tree
// ─────────────────────────────────────────────────────────────────────────────

fn bench_events_vs_tree(c: &mut Criterion) {
    let parser = build_json_parser();
    let tokens = json_tokens_large();
    let mut g = c.benchmark_group("parse_events_vs_tree");
    g.throughput(Throughput::Elements(tokens.len() as u64));

    g.bench_function("parse_tree", |b| {
        b.iter(|| {
            let mut ctx = DiagnosticsContext::new();
            parser.parse_tree(&tokens, &mut ctx)
        })
    });

    g.bench_function("parse_events", |b| {
        b.iter(|| {
            let mut ctx = DiagnosticsContext::new();
            parser.parse_events(&tokens, &mut ctx)
        })
    });

    g.finish();
}

// ─────────────────────────────────────────────────────────────────────────────
// 4. Long token list (repeat1 stress)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
enum RepRule { Root }
impl RuleId for RepRule {}

fn bench_repeat1_stress(c: &mut Criterion) {
    let profile = ResolvedProfile::simple("rep");
    let parser = grammar::<RepRule>()
        .rule(RepRule::Root, node(SyntaxKind::new("root"), repeat1(tok("WORD"))))
        .start(RepRule::Root)
        .compile(&profile)
        .unwrap();

    let mut g = c.benchmark_group("parse_repeat1");

    for n in [50usize, 500, 2000] {
        let tokens: Vec<Token> = (0..n).map(|_| t("WORD", "w")).collect();
        g.throughput(Throughput::Elements(tokens.len() as u64));
        g.bench_with_input(BenchmarkId::new("tokens", n), &tokens, |b, toks| {
            b.iter(|| {
                let mut ctx = DiagnosticsContext::new();
                parser.parse_tree(toks, &mut ctx)
            })
        });
    }

    g.finish();
}

// ─────────────────────────────────────────────────────────────────────────────
// 5. Deeply nested structures
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
enum NestedRule { Root, Expr }
impl RuleId for NestedRule {}

fn bench_deeply_nested(c: &mut Criterion) {
    // Grammar: expr = NUM | "{" expr "}"
    let profile = ResolvedProfile::simple("nested");
    let parser = grammar::<NestedRule>()
        .rule(NestedRule::Expr, node(SyntaxKind::new("expr"), one_of!(
            tok("NUM"),
            between(tok("LB"), ref_(NestedRule::Expr), tok("RB"))
        )))
        .rule(NestedRule::Root, ref_(NestedRule::Expr))
        .start(NestedRule::Root)
        .compile(&profile)
        .unwrap();

    let mut g = c.benchmark_group("parse_deeply_nested");

    for depth in [10usize, 50, 100] {
        // Build { { { … NUM … } } }
        let mut tokens: Vec<Token> = (0..depth).map(|_| t("LB", "{")).collect();
        tokens.push(t("NUM", "1"));
        tokens.extend((0..depth).map(|_| t("RB", "}")));

        g.throughput(Throughput::Elements(tokens.len() as u64));
        g.bench_with_input(BenchmarkId::new("depth", depth), &tokens, |b, toks| {
            b.iter(|| {
                let mut ctx = DiagnosticsContext::new();
                parser.parse_tree(toks, &mut ctx)
            })
        });
    }

    g.finish();
}

// ─────────────────────────────────────────────────────────────────────────────
// 6. Parser throughput scaling
// ─────────────────────────────────────────────────────────────────────────────

/// Build `[0, 1, 2, …, n-1]` as a JSON token stream.
fn json_flat_array(n: usize) -> Vec<Token<'static>> {
    let mut v = Vec::with_capacity(1 + n * 2 + 1);
    v.push(t("LBRACKET", "["));
    for i in 0..n {
        if i > 0 { v.push(t("COMMA", ",")); }
        v.push(t("NUMBER", "0"));
    }
    v.push(t("RBRACKET", "]"));
    v
}

fn bench_parse_throughput(c: &mut Criterion) {
    let parser = build_json_parser();
    let mut g = c.benchmark_group("parse_throughput");

    for n in [100usize, 500, 2000] {
        let tokens = json_flat_array(n);
        g.throughput(Throughput::Elements(tokens.len() as u64));
        g.bench_with_input(BenchmarkId::new("array_len", n), &tokens, |b, toks| {
            b.iter(|| {
                let mut ctx = DiagnosticsContext::new();
                parser.parse_tree(toks, &mut ctx)
            })
        });
    }

    g.finish();
}

// ─────────────────────────────────────────────────────────────────────────────
// 7. vs serde_json  — end-to-end JSON parsing comparison
//
// Compares the full rb pipeline (tokenize → parse_tree) with serde_json's
// from_str::<serde_json::Value> on the same JSON byte strings.
//
// rb_parser is a *structure-preserving* lossless CST parser (retains all
// trivia, node boundaries and field names), while serde_json is a value
// deserialiser that discards structure; the comparison shows the cost gap
// between the two design goals.
// ─────────────────────────────────────────────────────────────────────────────

/// Minimal JSON tokenizer for the pipeline side of the comparison.
fn json_tokenizer_pipeline() -> Tokenizer {
    let mut t = Tokenizer::new();
    t.add_block_scanner("\"", "\"", "STRING", None, true, false, false);
    t.add_regex_scanner(r"^\d+(?:\.\d+)?(?:[eE][+-]?\d+)?", "NUMBER", None).unwrap();
    t.add_keyword_scanner("TRUE",  &["true"]);
    t.add_keyword_scanner("FALSE", &["false"]);
    t.add_keyword_scanner("NULL",  &["null"]);
    t.add_symbol_scanner("{", "LBRACE",   None);
    t.add_symbol_scanner("}", "RBRACE",   None);
    t.add_symbol_scanner("[", "LBRACKET", None);
    t.add_symbol_scanner("]", "RBRACKET", None);
    t.add_symbol_scanner(":", "COLON",    None);
    t.add_symbol_scanner(",", "COMMA",    None);
    t
}

/// JSON string inputs of three sizes used by the vs_serde_json bench.
///
/// The sizes correspond roughly to the existing bench tiers:
///   small  (~40 B)  — a single flat object
///   medium (~700 B) — 50-key object with string and integer values
///   large  (~9 KB)  — array of 200 objects with four fields each
fn json_str_small() -> String {
    r#"{"name":"Alice","age":30,"active":true,"score":99.5}"#.to_string()
}

fn json_str_medium() -> String {
    let pairs: Vec<String> = (0..50)
        .map(|i| format!(r#""key{i}":"value{i}","num{i}":{i}"#))
        .collect();
    format!("{{{}}}", pairs.join(","))
}

fn json_str_large() -> String {
    let obj = |i: usize| format!(
        r#"{{"id":{i},"name":"item{i}","value":{v},"active":true}}"#,
        v = i * 17
    );
    let objs: Vec<String> = (0..200).map(obj).collect();
    format!("[{}]", objs.join(","))
}

fn bench_vs_serde_json(c: &mut Criterion) {
    let tokenizer = json_tokenizer_pipeline();
    let parser    = build_json_parser();

    let small  = json_str_small();
    let medium = json_str_medium();
    let large  = json_str_large();

    let mut g = c.benchmark_group("vs_serde_json");

    for (name, input) in [("small", &small), ("medium", &medium), ("large", &large)] {
        g.throughput(Throughput::Bytes(input.len() as u64));

        // ── rb pipeline ───────────────────────────────────────────────────────
        // Includes tokenization so the comparison is honest end-to-end timing.
        g.bench_with_input(
            BenchmarkId::new("rb_pipeline", name),
            input,
            |b, inp| {
                b.iter(|| {
                    let tokens = tokenizer.tokenize(inp).unwrap();
                    let mut ctx = DiagnosticsContext::new();
                    parser.parse_tree(&tokens, &mut ctx)
                })
            },
        );

        // ── serde_json ────────────────────────────────────────────────────────
        g.bench_with_input(
            BenchmarkId::new("serde_json", name),
            input,
            |b, inp| b.iter(|| serde_json::from_str::<serde_json::Value>(inp).unwrap()),
        );
    }

    g.finish();
}

// ─────────────────────────────────────────────────────────────────────────────
// Registration
// ─────────────────────────────────────────────────────────────────────────────

criterion_group!(
    benches,
    bench_parse_json,
    bench_parse_expr_pratt,
    bench_events_vs_tree,
    bench_repeat1_stress,
    bench_deeply_nested,
    bench_parse_throughput,
    bench_vs_serde_json,
);
criterion_main!(benches);
