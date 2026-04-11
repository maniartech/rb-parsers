# Implementation Plan

**Based on**: Framework Audit — April 2026
**Goal**: World-class parsing framework — industry-standard ergonomics, ≥ 20 MB/s
production throughput, IDE/LSP capable.

This document translates the audit findings into a sequenced delivery plan organised
into milestones. Each milestone is independently shippable and ends with a verified,
benchmarked, tested commit.

---

## Milestone 0 — Foundation is clean ✅

All P0 correctness bugs (A1–A3, B1–B3, C1–C5, C9, D1) are resolved.
**306 tests passing, 0 failures.** The codebase is ready for P1+ work.

---

## Milestone 1 — Performance Sprint I: Token Lifetime

**Target**: Remove the dominant allocation source. Break the 7–12× benchmark gap.
**Issues**: B4, B6, A8
**Expected outcome**: 40–60% throughput gain on all benchmarks.
**Estimated effort**: 2–3 weeks (B4 is a breaking API change).

### Work items (in order)

#### 1.1 — B4: `Token<'src>` with `Cow<'src, str>`

**File**: `crates/rb_tokenizer/src/tokens/token.rs`

```rust
pub struct Token<'src> {
    pub token_type:     &'static str,
    pub token_sub_type: Option<&'static str>,
    pub value:          std::borrow::Cow<'src, str>,
    pub span:           SourceSpan,
}
```

Migration checklist:
- [ ] Add lifetime `'src` to `Token`
- [ ] Propagate `'src` to `Tokenizer<'src>` and `TokenStream<'src>`
- [ ] Update all `Scanner::scan` implementations to return `Cow::Borrowed`
      for non-transforming scanners (keywords, operators, identifiers, numbers,
      delimited blocks with `include_delimiters: true`)
- [ ] Keep `Cow::Owned` only for escape-processing scanners (string literal internals)
- [ ] Update `CompiledParser::parse_tree` to accept `&[Token<'_>]`
- [ ] Update all tests and benchmarks; run `cargo test --workspace`
- [ ] Run `cargo bench --workspace` and record new baseline

#### 1.2 — B6: Remove `keyword.clone()` / `operator.clone()` in scanner hot paths

**Files**: `crates/rb_tokenizer/src/scanners/keyword_scanner.rs`,
           `crates/rb_tokenizer/src/scanners/operator_scanner.rs`

Change `value: keyword.to_string()` / `value: operator.to_string()` to
`value: Cow::Borrowed(keyword)` / `value: Cow::Borrowed(operator)`.
This is a straight mechanical change after 1.1 lands.

#### 1.3 — A8: `SpanLabel::message: Option<Cow<'static, str>>`

**File**: `crates/rb_common/src/spans.rs`

Replace `Option<String>` with `Option<Cow<'static, str>>`. Update construction
call sites from `.to_string()` to `Cow::Borrowed(...)` for literals.

### Exit criteria

- All tests pass
- `vs_serde_json` benchmark shows ≥ 40% improvement on `large` input
- No `String` allocation in the token-construction hot path (verify with `heaptrack`
  or similar on the large JSON bench)

---

## Milestone 2 — Performance Sprint II: Position Tracking and Scanner Dispatch

**Target**: Close the remaining 3–6× gap. Reach ≥ 10 MB/s.
**Issues**: B5, B7, B8, C6, C8
**Estimated effort**: 3–4 weeks.

### Work items (in order)

#### 2.1 — B5: Lazy `SourceMap` for line/column lookup

**File**: `crates/rb_tokenizer/src/tokenizers/tokenizer.rs`

Replace `advance_cursor` character loop with:
1. Build `SourceMap` once from the full source string using `memchr::memchr_iter`.
2. Track only `byte_offset` in the hot loop.
3. Call `source_map.position_of(byte_offset)` lazily when constructing `SourceSpan`.

Add `memchr` as a direct dependency in `crates/rb_tokenizer/Cargo.toml`.

#### 2.2 — B7: First-byte dispatch table

**File**: `crates/rb_tokenizer/src/tokenizers/tokenizer.rs`

Add `fn first_bytes(&self) -> Option<&[u8]>` to the `Scanner` trait (default: `None`).
Implement for `SymbolScanner`, `KeywordScanner`, `OperatorScanner`, `BlockScanner`
(first byte of the open delimiter).

Build a `[Vec<usize>; 256]` scanner-index table on `Tokenizer` construction.
Hot loop: try only `first_byte_table[input[0]]` scanners, then fallback list.

#### 2.3 — C6: Pre-build rule-key map in `Grammar::compile()`

**File**: `crates/rb_parser/src/grammar/combinator.rs`

Pre-compute `HashMap<String, usize>` of rule keys before calling `resolve_refs`.
Pass it as a parameter instead of calling `format!("{:?}", rule_id)` per Ref node.

#### 2.4 — C8: Flat children arena in `CstTree`

**File**: `crates/rb_parser/src/cst.rs`, `crates/rb_parser/src/strategy.rs`

Replace `CstNode::children: Vec<CstNodeChild>` with `(children_start: u32, children_len: u16)`.
Add `CstTree::children_store: Vec<CstNodeChild>`.

`CstBuildingStrategy` already uses an internal arena (`children_arena`); the change
is to keep children in that flat store rather than draining into per-node `Vec`s.

Update `walk_node`, `tokens_of`, and `CstNode::field()` to slice `&children_store`.
This simultaneously resolves the C1 cloning issue.

### Exit criteria

- All tests pass
- `vs_serde_json` large benchmark reaches ≤ 2× gap (≥ 10 MB/s)
- `cargo bench` baseline committed as `perf/milestone-2`

---

## Milestone 3 — Trivia Filtering and Streaming Pipeline

**Target**: Reduce memory peak, improve source-code grammar throughput, lay the
streaming foundation.
**Issues**: E1, E2
**Estimated effort**: 2–3 weeks. **E1 is independent of M1/M2; E2 requires M1 first.**

### Work items

#### 3.1 — E1: `TokenizerConfig::drop_token_types`

**File**: `crates/rb_tokenizer/src/tokenizers/tokenizer.rs`

```rust
pub struct TokenizerConfig {
    pub tokenize_whitespace: bool,
    pub drop_token_types: std::collections::HashSet<&'static str>,
    // ...
}
```

After a scanner match: check `drop_token_types` before constructing a `Token`.
If the type is in the set, advance `byte_offset` by `consumed_len` and `continue`.

Add `TriviaMode` enum for higher-level control (`Drop` / `Attach`).
Add convenience `config.drop_trivia(&["WHITESPACE", "LINE_COMMENT", "BLOCK_COMMENT"])`.

**Can be implemented before or in parallel with Milestone 1.**

#### 3.2 — E2: `TokenSource` trait and `BufferedTokenSource`

**File**: new `crates/rb_tokenizer/src/token_source.rs`,
          `crates/rb_parser/src/lib.rs`, `crates/rb_parser/src/grammar/combinator.rs`

After M1 (Token has `'src` lifetime):

1. Define `TokenSource` trait in `rb_tokenizer`.
2. Implement `SliceTokenSource<'a>` (wraps `&'a [Token<'src>]` + cursor index).
3. Implement `BufferedTokenSource<I>` (wraps `Iterator<Item = Token<'src>>` +
   `VecDeque<Token<'src>>` sliding window).
4. Migrate `ParseContext` from `(tokens: &[Token], pos: usize)` to hold a
   `&mut dyn TokenSource`.
5. Keep `parse_tree(&[Token<'_>])` as a convenience that constructs `SliceTokenSource`.
6. Add `parse_streaming(token_iter: impl Iterator<Item = Token<'_>>)` using
   `BufferedTokenSource`.

### Exit criteria

- E1: `lex_source_code` benchmark shows ≥ 8% improvement when `drop_token_types`
  is populated
- E2: `parse_throughput` benchmark shows ≤ 5% overhead vs. slice mode (streaming
  should be within noise); peak RSS for large input reduced by ~30%

---

## Milestone 4 — Grammar Completeness (P2)

**Target**: Unlock JS/Python/Ruby-complexity grammars.
**Issues**: C10, C11, C13, C14, C15, C16
**Estimated effort**: 2–4 weeks.

### Work items

#### 4.1 — C13: `PrattOp::token_sub_type`

**File**: `crates/rb_parser/src/grammar/combinator.rs`

Add `token_sub_type: Option<&'static str>` to `PrattOp`. Update `matches_op` predicate.
Expose in `pratt_op!` macro / builder API.

#### 4.2 — C14: `CompiledParser::with_recovery(config)`

**File**: `crates/rb_parser/src/lib.rs`

```rust
impl CompiledParser {
    pub fn with_recovery(mut self, config: RecoveryConfig) -> Self {
        self.recovery = config;
        self
    }
}
```

One-liner change; high leverage for test ergonomics.

#### 4.3 — C10: `look(expr)` and `not(expr)` combinators

**File**: `crates/rb_parser/src/grammar/combinator.rs`

Add `RuleExpr::Lookahead { positive: bool, inner: Box<RuleExpr<R>> }` variant.
Implement in `eval()`: attempt `inner`, then unconditionally restore the token cursor.
Succeed or fail based on `positive`.

#### 4.4 — C11: `take_until(end)` / `until(content, end)` combinators

**File**: `crates/rb_parser/src/grammar/combinator.rs`

Add `RuleExpr::TakeUntil { end: Box<RuleExpr<R>> }` variant.
`eval`: loop consuming one token at a time until `end` matches at current position
(without consuming `end`).

#### 4.5 — C15: FIRST set computation in `Grammar::compile()`

**File**: `crates/rb_parser/src/grammar/combinator.rs`

Compute `first_set: HashMap<usize, HashSet<&'static str>>` (expr index → accepted
first token types) during the compile step. Store on `CompiledGrammar`.

Expose via `CompiledParser::first_tokens_at_rule(rule_id)` for grammar diagnostics.

#### 4.6 — C16: First-token dispatch in `one_of!`

**File**: `crates/rb_parser/src/grammar/combinator.rs`

After C15: before the `one_of` sequential scan, check `first_set[branch]` against the
current token type and skip branches whose FIRST set does not contain it. Degrades
gracefully to sequential for branches with no computable FIRST set.

### Exit criteria

- New test: arithmetic grammar with `==`, `!=`, `+`, `-`, `*`, `/` all using
  `token_type = "Op"` and distinct `token_sub_type` values parses correctly
- New test: `not(keyword) ~ ident` rejects keyword tokens at identifier positions
- New test: `take_until(tok("RBRACE"))` correctly scans a template literal body
- All existing tests continue to pass

---

## Milestone 5 — Visitor Ecosystem

**Target**: Rich, ergonomic tree consumption API in the `visitor` crate.
**Issues**: E3, E4, E5, E6, E7 (partial)
**Estimated effort**: 2–3 weeks. **Independent of M1–M4.**

### Work items

#### 5.1 — E6: `StreamVisitor` / event-streaming adapter

**File**: `crates/visitor/src/lib.rs`, or new `crates/visitor/src/stream.rs`

Expose `ParseEvent`-level access as a stable public trait. Implement
`StreamVisitorStrategy<V>` as a `ParseStrategy` adapter so `StreamVisitor`
implementations can be passed to `parse_with_strategy`.

**Start here** — it re-uses existing infrastructure with the least new code.

#### 5.2 — E3: `KindVisitor`

**File**: `crates/visitor/src/lib.rs`

Builder-style API (`KindVisitor::new().on_enter(kind, handler)`).
Implements `TreeVisitor`. O(handlers_for_kind) dispatch; for typical grammars this
is O(1).

#### 5.3 — E4: `SyntaxCursor`

**File**: `crates/rb_parser/src/cst.rs`, `crates/visitor/src/lib.rs`

Add `parent_id: Option<SyntaxNodeId>` to `CstNode` (populated in
`CstBuildingStrategy` on `NodeEnd`, pointing to the stack frame above).

Implement `SyntaxCursor` with `parent()`, `first_child()`, `next_sibling()`,
`prev_sibling()`, `children()`, `child_of_kind()`, `field()`, `ancestor_of_kind()`.

#### 5.4 — E5: `ContextualWalker`

**File**: `crates/visitor/src/lib.rs`

`ContextualVisitor` trait + `ContextualWalker` that maintains `Vec<SyntaxNodeId>`
ancestor stack internally.

#### 5.5 — E7: `TreeTransform` trait + `TransformAction` (stub only)

**File**: `crates/visitor/src/lib.rs`

Define the trait and enum now to establish the stable API shape. Leave
`CstTree::apply_transform` as `todo!()` behind a `#[doc(hidden)]` flag until C8
(flat arena) is complete. Full implementation follows in Milestone 6.

### Exit criteria

- `DepthFirstWalker` + `KindVisitor` example in the `visitor` crate's doc tests
- `SyntaxCursor::ancestor_of_kind` integration test over a JSON parse tree
- `StreamVisitor` example: count all `NUMBER` tokens without building a CST

---

## Milestone 6 — Architecture Cleanup (P3)

**Target**: Remove structural debt before API stabilization.
**Issues**: A4, A6, A7, C17, C20, D3, D4, C3, C7
**Estimated effort**: 2–3 weeks. **Do before 1.0 API freeze.**

### Work items (can be parallelised across contributors)

| # | Issue | File | Description |
|---|---|---|---|
| 6.1 | A4 | `rb_common/recovery.rs`, `rb_parser/profiles.rs` | Unify duplicate `RecoveryConfig` structs; `rb_parser` imports from `rb_common` |
| 6.2 | A6/A7 | `rb_common/render.rs` | Implement `RendererSuitability` registry; pass `RenderRequest` env snapshot to `render()` |
| 6.3 | C17 | `rb_parser/profiles.rs` | Implement `ProfileCatalog` registry for profile guard system |
| 6.4 | C20 | new `rb_common/source_registry.rs` | `SourceIdRegistry` with collision detection for multi-file setups |
| 6.5 | D3 | `rb_parser/lib.rs` | Implement `std::error::Error` for `CompiledParser` error type |
| 6.6 | D4 | all public enums | Add `#[non_exhaustive]` to all public enums across all crates |
| 6.7 | C3 | `rb_parser/grammar/combinator.rs` | Remove dead `let _stack` allocation in `check_left_recursion` |
| 6.8 | C7 | `rb_parser/lib.rs` | Remove redundant `unsafe impl Send/Sync` |

#### 6.9 — E7 (complete): `CstTree::apply_transform`

**After C8 (Milestone 2)**: implement `apply_transform` body using the flat arena.
Copy-on-write: unmodified children slices are referenced in-place; modified nodes
are written to a fresh `children_store`.

### Exit criteria

- `cargo test --workspace` passes with single `RecoveryConfig` in `rb_common`
- No duplicate struct definitions found by `grep RecoveryConfig`
- All public enums have `#[non_exhaustive]`

---

## Milestone 7 — Ecosystem and Pre-release Polish (P4/P5)

**Target**: Framework is externally consumable and CI-protected.
**Issues**: D5, D6, D7, D8, D9, D10, B9, B10, B11, C18, C19
**Estimated effort**: 3–4 weeks.

### Work items

| # | Issue | Description |
|---|---|---|
| 7.1 | C18 | `CstToken::text<'a>(&self, source: &'a str) -> &'a str` helper |
| 7.2 | D7 | `CstTree::to_sexpr() -> String` and `impl Display` |
| 7.3 | B9 | `Tokenizer::scanner_count()` / `scanner_names()` introspection |
| 7.4 | B10 | Document `add_scanner_with_priority` ordering contract |
| 7.5 | B11 | `tokenize_from(input, start_byte, start_position)` for incremental re-scan |
| 7.6 | D6 | `Scanner::clone_box()` + `impl Clone for Tokenizer` |
| 7.7 | D5 | `[features] serde` gate on `Token`, `CstTree`, `Diagnostic`, `SourceSpan` |
| 7.8 | C19 | `TriviafreeVisitor`, `AstBuilder<N>` reference implementations in `visitor` |
| 7.9 | D8 | `fuzz/fuzz_targets/tokenizer.rs` + `fuzz/fuzz_targets/parser.rs` |
| 7.10 | D9 | Add `#![warn(missing_docs)]` to all crates and fix warnings |
| 7.11 | D10 | CI benchmark baseline tracking with `github-action-benchmark` |
| 7.12 | A5 | Update `DiagnosticSeverity` deprecation attr with `since` + `note` |

### Exit criteria

- `cargo fuzz build` succeeds with no errors
- `cargo doc --workspace --no-deps` produces zero warnings
- CI benchmark step passes on a clean PR

---

## Milestone 8 — Incremental Re-parse (P4, hardest single item)

**Target**: Sub-100 ms re-parse on a 10k-line file for a single-line edit.
**Issues**: C12, B11 (reuse from M7)
**Estimated effort**: 4–8 weeks. **Prerequisite: M1, M2, M3 (streaming).**

This is the tree-sitter-level problem. The approach:

1. **Checkpoint mechanism**: extend `ParseContext` with a `reuse_table: Option<&NodeReuseTable>`.
   `NodeReuseTable` maps `(byte_offset, SyntaxKind)` → `CstNode` from the previous parse.

2. **`ParseStrategy::can_reuse(node: &CstNode, edit: &TextEdit) -> bool`**:
   default `false`. `CstBuildingStrategy` implements it by checking whether the node's
   span does not intersect the edit region.

3. **Edit application**: `CstTree::apply_edit(edit: TextEdit) -> EditedTree` produces a
   new tree with nodes in the unchanged regions re-used by reference (cheap `Arc` clone
   of the arena slice).

4. **Re-tokenize** only the changed line range using `tokenize_from` (B11).

5. **Re-parse** only from the earliest changed token, using `NodeReuseTable` to skip
   subtrees that are unaffected.

This milestone is independent and should be planned as its own sprint with dedicated
review capacity.

### Exit criteria

- Benchmark: re-parse a 1000-line JSON file after inserting one character at line 500
  in ≤ 2 ms
- All unit tests pass with incremental and full-parse producing identical `CstTree`
  output for all existing fixtures

---

## Dependency Graph (summary)

```
M0 (P0 fixed ✅)
  └─ M1 (B4 Cow token) ──────────────────────────────────────────────┐
      └─ M2 (B5 SourceMap, B7 dispatch, C8 arena) ──────────────────┐│
          └─ M3.2 (E2 streaming, requires M1) ─────────────────────┐││
  M3.1 (E1 trivia filter, independent of M1) ─────────────────────┐│││
  M4 (C10/C11/C13/C14 grammar API, independent of M1/M2) ─────────┤│││
  M5 (E3–E7 visitor, independent of M1–M4) ─────────────────────── ┤│││
      └─ M5 E7 complete ──── requires M2 (C8 arena) ─────────────── ┤│││
  M6 (P3 architecture cleanup) ─ after API is stabilised ─────────── ┤│││
  M7 (P4/P5 polish) ─ before external release ───────────────────── ┤│││
  M8 (C12 incremental, hardest) ─ requires M1+M2+M3 ───────────────────┘│││
                                                                         └┴┴┘
```

---

## Priority-ordered backlog (for sprint planning)

| Sprint | Issues | Rationale |
|---|---|---|
| 1 | **B4, B6, A8** | Biggest single performance gain; unblocks everything downstream |
| 2 | **B5, B7, C8** | Close remaining throughput gap; reach 10 MB/s |
| 3 | **E1** (parallel with S1/S2), **C13, C14** | Quick ergonomics wins; fix broken Pratt |
| 4 | **C10, C11** | Complete grammar expressiveness for real languages |
| 5 | **E6, E3, E4, E5** | Visitor ecosystem; low risk, high ergonomics impact |
| 6 | **E2** (after B4), **B11** | Streaming pipeline + incremental tokenizer restart |
| 7 | **A4, D4, C20, D3** | Architecture cleanup before API freeze |
| 8 | **D5, D6, C18, C19, D7, D8, D9, D10, B9, B10, A5** | Polish and release readiness |
| 9 | **C15, C16** | FIRST-set dispatch (significant compiler work, high payoff for large grammars) |
| 10 | **C12** | Incremental re-parse (own dedicated sprint) |
