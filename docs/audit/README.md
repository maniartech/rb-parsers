# Framework Audit — April 2026

**Scope**: Full codebase review of `rb_common`, `rb_tokenizer`, and `rb_parser` against
the framework objectives defined in
[`docs/requirements/framework-objectives.md`](../requirements/framework-objectives.md).

**Date**: 2026-04-09
**Status**: All P0 correctness bugs resolved — 12 of 12 fixed

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
| [Implementation Plan](implementation-plan.md) | Sequenced milestones, sprint backlog, and dependency graph | Living planning document |

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
| E1 | rb_tokenizer | Trivia token types not configurable — trivia always flows into the parser | P1 |
| E2 | rb_tokenizer / rb_parser | Full `Vec<Token>` materialization — parser cannot consume tokens lazily | P1 |
| E3 | visitor | No kind-dispatched visitor — grammar authors repeat `if node.kind == …` everywhere | P4 |
| E4 | visitor | No cursor / navigation API — subtree traversal requires a full tree walk | P4 |
| E5 | visitor | No path-aware visitor — context-sensitive analysis requires manual ancestor tracking | P4 |
| E6 | visitor | No event-streaming visitor — every analysis pass forces full CST materialization | P4 |
| E7 | visitor | No mutable / transform visitor — CST rewrites require manual tree reconstruction | P4 |

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

### Theme 5 — Trivia, Streaming, and Pipeline Architecture

Three issues form a coherent pipeline-level improvement track: E1 (trivia filtering)
lets the tokenizer discard irrelevant tokens before they touch the heap; E2 (streaming
`TokenSource`) eliminates full-file token materialization so the parser and tokenizer
run concurrently rather than sequentially; B11 (tokenize_from) closes the incremental
gap at the tokenizer level. The dependency chain is strict: **E1 is independent → B4
(Cow token) must land first → then E2 wires in the streaming interface → then B11
adds the offset restart API.** Taken together they represent the path from 2.3 MB/s
to ≥ 20 MB/s for realistic source-code inputs. See [P1](p1-performance.md).

### Theme 6 — Visitor Ecosystem and Code Consumption Ergonomics

The `visitor` crate is currently a 7-line re-export shim. Five complementary visitor
patterns (E3–E7) address the full spectrum of downstream consumers:

- **E3 Kind-dispatch** — removes boilerplate `match node.kind` in every visitor;
  prerequisite for any grammar tooling beyond toy examples.
- **E4 Cursor API** — enables IDE-style point navigation without full-tree walks;
  the foundation for C19 (AST lowering) and LSP cursor features.
- **E5 Path-aware** — supplies the ancestor chain to every hook; enables context-
  sensitive analysis without manually maintained stacks.
- **E6 Event-streaming** — exposes the existing `ParseStrategy` infrastructure as
  a user-facing API, enabling zero-CST single-pass analysis at parse speed.
- **E7 Transform** — enables in-place CST rewrites for desugaring and macro
  expansion; depends on C8 (flat arena) for an efficient implementation.

E3, E4, E5, and E6 are independently implementable with no P1 or P2 blockers.
E7 should be stubbed (trait + enum) now and fully implemented after C8.

---

## Production Language & IDE Language Server Readiness

This section answers: **"what do I need to fix before I can build X?"**

### Capability Tiers

| Target use case | Status today | Blocking issues |
|---|---|---|
| Config / DSL / TOML-like grammars | ✅ **Ready now** | None critical |
| Full programming language (JSON-like complexity) | ✅ **Ready now** | — |
| Full programming language (JS / Python / Ruby complexity) | ❌ **Not ready** | C10, C13, B4, B5 |
| Compiler front-end (batch, no IDE) | ⚠️ Close — fix P1+P2 first | C10, C11, C13, C14, B4, B5, B7, E1 |
| IDE language server (LSP) | ❌ **Not ready** | All P1+P2 + C12, B11, D5, D6, C19, E2, E4, E6, and LSP wire protocol (absent entirely) |

---

### Full Programming Language — Functional Blockers (P2)

These block **any non-trivial grammar**. Fix before attempting a real language.

| Issue | File | Why it blocks a real PL |
|---|---|---|
| **C10** — no `look()`/`not()` lookahead | [p2-missing-api.md](p2-missing-api.md) | Cannot distinguish keywords from identifiers; cannot express `!keyword ~ ident`; all contextual disambiguation requires escape to raw `ParseFn` |
| **C13** — `PrattOp` ignores `token_sub_type` | [p2-missing-api.md](p2-missing-api.md) | All binary operators share the same binding power → `a + b == c + d` is misparsed; a correct arithmetic grammar is impossible |
| **C11** — no `take_until()` | [p2-missing-api.md](p2-missing-api.md) | No template literals, heredocs, raw strings, or embedded expressions; workaround is O(tokens) recursive `many(not(end) ~ any)` |
| **C14** — no `with_recovery()` builder | [p2-missing-api.md](p2-missing-api.md) | Recovery config silently ignored; no per-parse-session control; tests cannot disable recovery |

**Estimated effort**: 2–4 weeks.

---

### Full Programming Language — Performance Blockers (P1)

Current pipeline throughput: ~**2.3 MB/s**. A responsive language tool needs ≥ 20 MB/s;
a language server needs ≥ 50 MB/s. The gap is ~10–22×. Root causes, in priority order:

| Issue | File | Expected gain |
|---|---|---|
| **B4** — `String` per token (heap alloc per match) | [p1-performance.md](p1-performance.md) | 40–60% reduction alone |
| **B5** — O(bytes/token) line/col tracking | [p1-performance.md](p1-performance.md) | 25–35% after B4 |
| **B7** — O(scanners) dispatch per byte position | [p1-performance.md](p1-performance.md) | 15–30% in grammars with > 8 scanners |
| **E1** — trivia tokens flow through full pipeline | [p1-performance.md](p1-performance.md) | 5–15% on source-code inputs; enables `TriviaMode::Drop` |
| **E2** — full `Vec<Token>` materialization before parse | [p1-performance.md](p1-performance.md) | 10–20% additional on top of B4+B5; eliminates peak token-array allocation |

**Estimated effort**: 3–5 weeks. B4 is a breaking API change (adds lifetime to `Token`).

---

### IDE Language Server — Additional Blockers (P4)

Even after P1+P2, an LSP requires infrastructure that does not yet exist:

| Issue | File | Why the LSP cannot function without it |
|---|---|---|
| **C12** — no incremental re-parse | [p4-p5-ecosystem-polish.md](p4-p5-ecosystem-polish.md) | Every keystroke re-parses the entire file; a 10k-line file = hundreds of ms per edit |
| **B11** — no `tokenize_from(offset)` | [p4-p5-ecosystem-polish.md](p4-p5-ecosystem-polish.md) | Same problem at tokenizer level; no checkpoint to restart from |
| **D5** — no `serde` feature | [p4-p5-ecosystem-polish.md](p4-p5-ecosystem-polish.md) | CST cannot be serialized → no disk cache, no cross-process LSP transport |
| **D6** — `Tokenizer` not `Clone` | [p4-p5-ecosystem-polish.md](p4-p5-ecosystem-polish.md) | Cannot tokenize multiple files in parallel on a thread pool |
| **C19** — no `AstLoweringStrategy` impl | [p4-p5-ecosystem-polish.md](p4-p5-ecosystem-polish.md) | LSP needs a typed AST for hover/completion/rename; manually lowering CST per project is not viable |
| **C18** — no `CstToken::text()` | [p4-p5-ecosystem-polish.md](p4-p5-ecosystem-polish.md) | Every LSP feature (hover, go-to-def, rename) duplicates manual span arithmetic |
| **E2** — no streaming `TokenSource` | [p1-performance.md](p1-performance.md) | File-at-a-time tokenization; streaming is required for responsive large-file editing |
| **E4** — no cursor / navigation API | [p4-p5-ecosystem-polish.md](p4-p5-ecosystem-polish.md) | IDE cursor-to-node mapping, hover, and go-to-definition are impossible without O(1) parent navigation |
| **E6** — no event-streaming visitor | [p4-p5-ecosystem-polish.md](p4-p5-ecosystem-polish.md) | Syntax highlighting and symbol indexing must materialize a full CST; streaming is required for < 16 ms response |
| *(absent)* — **LSP wire protocol** | — | Zero `textDocument/hover`, `textDocument/completion`, etc. Requires integrating a crate like `tower-lsp`; this layer is entirely absent |

**Estimated effort**: 4–8 weeks for C12 alone (incremental re-parse is the hardest
single item in this list — analogous to what tree-sitter spent years on). C18/D5/D6/C19
together: ~2–3 weeks. LSP wire layer: ~6–12 weeks.

---

### Total Effort Estimate to LSP-Capable

| Layer | Issues | Rough effort |
|---|---|---|
| Correct grammar expressiveness | C10, C11, C13, C14 | 2–4 weeks |
| Performance (tolerable latency) | B4, B5, B7, E1 | 3–5 weeks |
| Streaming pipeline | E2 (after B4) | 1–2 weeks |
| Visitor ecosystem | E3, E4, E5, E6 (independent); E7 after C8 | 2–3 weeks |
| Incremental re-parse foundation | C12, B11 | 4–8 weeks |
| CST-to-AST + serde + thread safety | C19, D5, D6, C18 | 2–3 weeks |
| LSP wire protocol integration | `tower-lsp` wrapper, workspace indexer | 6–12 weeks |
| **Total** | | **~20–37 weeks** |

**Recommended sequencing**: Fix P1+P2 first → ship a formatter/linter/compiler front-end
as a milestone → then tackle C12 (incremental re-parse) as its own dedicated milestone
before building the LSP layer.
