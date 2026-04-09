# Framework Audit — April 2026

**Scope**: Full codebase review of `rb_common`, `rb_tokenizer`, and `rb_parser` against
the framework objectives defined in
[`docs/requirements/framework-objectives.md`](../requirements/framework-objectives.md).

**Date**: 2026-04-09
**Status**: Initial findings — no issues resolved yet

---

## Purpose

The framework goal is a world-class parsing framework: militarily robust, production
competitive on benchmarks, and the most ergonomic in its class. This audit identifies
every place where the current implementation falls short of that goal, organized by
priority so work can be tracked and sequenced.

---

## Finding Files

| File | Coverage | Priority |
|---|---|---|
| [P0 — Correctness Bugs](p0-correctness-bugs.md) | Defects that produce wrong output or silently incorrect behavior today | Fix before any other work |
| [P1 — Performance](p1-performance.md) | Hot-path inefficiencies causing the 7–12× benchmark gap vs. serde_json | Fix next — determines competitive position |
| [P2 — Missing Fundamental API](p2-missing-api.md) | Grammar primitives and parser features without which grammar authors are blocked on real languages | Required before 1.0 |
| [P3 — Architecture & Design](p3-architecture.md) | Structural problems that will compound as the codebase grows | Fix before API stabilization |
| [P4–P5 — Ecosystem & Polish](p4-p5-ecosystem-polish.md) | Missing ecosystem features, ergonomics, and documentation | Required before production release |

---

## Summary Table

| ID | Layer | Title | Priority |
|---|---|---|---|
| A1 | rb_common | PlainRenderer discards the error-line marker | P0 |
| A2 | rb_common | `DefaultHasher` used for profile ID — not stable across runs | P0 |
| A3 | rb_common | `SourceSpan::merge` of two UNKNOWN spans produces a valid-looking span | P0 |
| A4 | rb_common | Two incompatible `RecoveryConfig` structs — one is dead | P3 |
| A5 | rb_common | `DiagnosticSeverity` deprecation alias has no migration path | P5 |
| A6 | rb_common | `RendererSuitability` / suitability registry does not exist | P3 |
| A7 | rb_common | `RenderRequest` carries env snapshot but `render()` does not receive it | P3 |
| A8 | rb_common | `SpanLabel::message` allocates String for static-str messages | P1 |
| B1 | rb_tokenizer | `Scanner::scan_with_context` derives `consumed_len` from `value.len()` — wrong for transformed values | P0 |
| B2 | rb_tokenizer | `ContextualScanner` silently invisible when `tokenize()` used instead of `tokenize_contextual()` | P0 |
| B3 | rb_tokenizer | `TokenizationError` carries no source position when returned from `tokenize()` | P0 |
| B4 | rb_tokenizer | `value: String` in `Token` — heap allocation per token | P1 |
| B5 | rb_tokenizer | `advance_cursor` walks every byte per token — O(bytes per token) line/col tracking | P1 |
| B6 | rb_tokenizer | `keyword.clone()` / `operator.clone()` inside scanner hot path | P1 |
| B7 | rb_tokenizer | Linear O(N) scanner dispatch per input position | P1 |
| B8 | rb_tokenizer | 12-arm `match` on `ScannerType` per position — branch-prediction hostile | P1 |
| B9 | rb_tokenizer | No `Tokenizer` introspection after construction | P5 |
| B10 | rb_tokenizer | `add_scanner_with_priority` semantics unclear and undocumented | P5 |
| B11 | rb_tokenizer | No `tokenize_slice` for incremental sub-range re-scanning | P4 |
| C1 | rb_parser | `CstTree::walk_node` clones every node's `Vec<CstNodeChild>` during traversal | P0 |
| C2 | rb_parser | `GrammarBuilder::rule` silently overwrites duplicate rules — `DuplicateRule` error never emitted | P0 |
| C3 | rb_parser | Dead `let _stack` allocation in `check_left_recursion` inner loop | P3 |
| C4 | rb_parser | `ParseContext::recovery` is never consulted — recovery behaviour split across two mechanisms | P0 |
| C5 | rb_parser | Unclosed error nodes get zero-width span indistinguishable from real zero-width nodes | P0 |
| C6 | rb_parser | `format!("{:?}", rule_id)` called for every node in every expression during compilation | P1 |
| C7 | rb_parser | Redundant `unsafe impl Send/Sync` already guaranteed by `Arc<dyn ParseFn: Send+Sync>` | P3 |
| C8 | rb_parser | `CstNode::children: Vec<CstNodeChild>` — per-node heap allocation; should be flat arena slice | P1 |
| C9 | rb_parser | `CstNode::direct_tokens()` includes trivia; `CstTree::tokens_of()` does not — inconsistent | P0 |
| C10 | rb_parser | No `look(expr)` (positive lookahead) or `not(expr)` (negative lookahead) combinators | P2 |
| C11 | rb_parser | No `take_until(tok)` / `until(end_expr)` combinator | P2 |
| C12 | rb_parser | No incremental / partial re-parse API | P4 |
| C13 | rb_parser | `PrattOp` only discriminates by `token_type`, not `token_sub_type` | P2 |
| C14 | rb_parser | No `CompiledParser::with_recovery(config)` builder | P2 |
| C15 | rb_parser | No grammar introspection (FIRST sets, accepted token types at position) | P2 |
| C16 | rb_parser | `one_of!` is O(N) sequential — no first-token dispatch | P2 |
| C17 | rb_parser | No `ProfileCatalog` registry to drive the profile guard system | P3 |
| C18 | rb_parser | `CstToken` has no `text<'a>(&self, source: &'a str) -> &'a str` helper | P5 |
| C19 | rb_parser | No `AstLoweringStrategy` implementations — trait defined but empty | P4 |
| C20 | rb_parser | No `SourceIdRegistry` — multi-file ID collisions are silent | P3 |
| D1 | workspace | `visitor` crate referenced in workspace structure does not exist on disk | P0 |
| D2 | rb_tokenizer | No tokenizer error catalog — errors are stringly typed | P3 |
| D3 | workspace | `GrammarError` implements `std::error::Error` but `CompiledParser` does not | P3 |
| D4 | workspace | Public enums missing `#[non_exhaustive]` — adding variants is semver-breaking | P3 |
| D5 | workspace | No `serde` feature flag on any public type | P4 |
| D6 | rb_tokenizer | `Tokenizer` is not `Clone` — cannot parallelize without rebuilding per thread | P4 |
| D7 | rb_parser | No `Display` / `to_sexpr()` on `CstTree` | P5 |
| D8 | workspace | No fuzz testing infrastructure | P4 |
| D9 | workspace | `#![deny(missing_docs)]` not set — public API incompletely documented | P5 |
| D10 | workspace | No criterion baseline tracking in CI — performance regresses silently | P4 |

---

## Cross-cutting Themes

### Theme 1 — Token Lifetime and Allocation

`Token::value: String` is the single highest-impact performance defect. Fixing it with
`Cow<'src, str>` naturally unblocks streamed tokenization (B11), parallel tokenization
(D6), and is the prerequisite for incremental parsing (C12). See [P1](p1-performance.md).

### Theme 2 — Compile-time Grammar Analysis

Computing FIRST sets and lookahead tables at `Grammar::compile()` time closes C10, C13,
C15, and C16 simultaneously, and is the foundation for incremental re-parsing. Without
it, every `one_of!` with more than 2 branches is inherently O(N). See [P2](p2-missing-api.md).

### Theme 3 — Recovery Configuration

`RecoveryConfig` exists in two places with different fields. One is dead. The parser
ignores the one attached to `CompiledParser` in favour of a function parameter. Until
this is unified, configuring recovery behaviour is unreliable. See [P3](p3-architecture.md).

### Theme 4 — Diagnostic Rendering

The `PlainRenderer` discards the error-line marker (A1) and the rendering system
architecture is incomplete (A6, A7). Diagnostics are the user's most visible interaction
with parse failures — these affect the framework's quality perception more than any
benchmark number. See [P0](p0-correctness-bugs.md).
