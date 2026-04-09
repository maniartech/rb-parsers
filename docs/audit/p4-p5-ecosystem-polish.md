# P4–P5 — Ecosystem & Polish

**Priority**: P4 — Ecosystem readiness (blocks external consumers, benchmarks, CI).
**Priority**: P5 — Polish (developer experience, documentation quality).

Neither tier blocks current correctness or performance work, but these gaps will become
friction when the crates are used outside this workspace or when contributors join.

**Back to**: [Audit Index](README.md)

---

## B9 · No `Tokenizer` introspection — cannot list scanners, clone, or serialize

**Layer**: `rb_tokenizer` · P4
**File**: `crates/rb_tokenizer/src/tokenizers/tokenizer.rs`

`Tokenizer` is an opaque builder/runner. There is no way to:

- Ask it how many scanners are registered (`scanner_count()`).
- Clone it to share a configured tokenizer across threads (see D6).
- Serialize/deserialize the configuration for caching or diagnostics.

This also means test utilities cannot inspect whether a tokenizer was correctly
configured without running it against a fixed input and inferring from the output.

**Suggested additions:**

```rust
impl Tokenizer {
    pub fn scanner_count(&self) -> usize { self.scanners.len() }
    pub fn scanner_names(&self) -> Vec<&str> { ... }   // requires Scanner::name()
}
```

---

## B10 · `add_scanner_with_priority` priority semantics are undocumented

**Layer**: `rb_tokenizer` · P5
**File**: `crates/rb_tokenizer/src/tokenizers/tokenizer.rs`

`add_scanner_with_priority(scanner, priority: i32)` exists but there is no documentation
explaining:

- What the valid range of priority values is.
- Whether higher or lower numbers win.
- Whether priority interacts with registration order (i.e. do equal-priority scanners
  use registration order as a tiebreak?).
- Whether `add_scanner` (without priority) inserts at a fixed default priority or at
  the end of the scanner list.

This leaves grammar authors guessing. Unexpected scanner ordering produces silent
tokenization differences.

**Fix:** Add a doc comment block to `add_scanner_with_priority` explaining the
ordering contract, the default priority for `add_scanner`, and at least one example
showing priority override in action.

---

## B11 · No `tokenize_slice` for incremental re-scanning

**Layer**: `rb_tokenizer` · P4
**File**: `crates/rb_tokenizer/src/tokenizers/tokenizer.rs`

`tokenize(input: &str)` always processes the full input from byte 0. There is no:

- `tokenize_from(input: &str, start_byte: usize, start_position: SourcePosition)` for
  re-scanning from a known-good checkpoint.
- `tokenize_range(input: &str, range: std::ops::Range<usize>)` for re-scanning a
  changed region.

Without this, an editor calling `tokenize()` on every keystroke re-scans the entire
file from scratch. For a 10,000-line file this means hundreds of milliseconds of
unnecessary tokenizer work per edit.

**Fix:** Accept an optional `offset: usize` parameter to `tokenize()` and initialise
the tokenizer cursor there rather than at 0. This is a small change with a large
payoff for incremental use cases.

---

## C12 · No incremental / partial re-parse API

**Layer**: `rb_parser` · P4
**File**: `crates/rb_parser/src/lib.rs`

`CompiledParser::parse(source: &str)` always re-parses from scratch. There is no:

- `parse_with_old_tree(source: &str, old_tree: &CstTree, edit: TextEdit)` for
  incremental parsing using the unchanged subtrees.
- `parse_range(source: &str, start: SourcePosition, end: SourcePosition)` for
  re-parsing a specific region.

This is a hard problem (comparable to tree-sitter's incremental parsing), but the
absence of any API hook for it means it cannot be added later without breaking changes.

**Short-term fix:** Add a lifecycle hook `ParseStrategy::can_reuse_node()` that
strategies can implement to indicate whether a previously-built subtree can be
reused. Provide a default `false` implementation so behaviour is unchanged. This
establishes the extension point without requiring a full incremental engine.

---

## C18 · `CstToken` missing `text<'a>(&self, source: &'a str) -> &'a str` helper

**Layer**: `rb_parser` · P5
**File**: `crates/rb_parser/src/cst.rs`

After parsing, callers routinely need the source text of a token:

```rust
// Current — requires manual span arithmetic
let token = tree.token(token_id);
let text = &source[token.span.start.byte_offset..token.span.end.byte_offset];
```

There is no `CstToken::text(source: &str)` helper. Every call site duplicates this
span-slicing pattern, which is both verbose and error-prone (off-by-one on a half-open
vs closed interval).

**Fix:**

```rust
impl CstToken {
    pub fn text<'a>(&self, source: &'a str) -> &'a str {
        let start = self.span.start.byte_offset;
        let end   = self.span.end.byte_offset;
        &source[start..end]
    }
}
```

---

## C19 · No `AstLoweringStrategy` implementations provided

**Layer**: `rb_parser` · P4
**File**: `crates/rb_parser/src/visitors.rs` (or planned location)

The CST-to-AST lowering pattern is central to the framework's value proposition
(`framework-objectives.md`). The visitor trait exists but there are no provided
implementations:

- No `FilteringVisitor<Inner>` that skips trivia nodes automatically.
- No `AstBuilder<Node>` that maps `CstNode` by kind to user-defined AST types.
- No `SpanCollector` that collects the spans of named nodes.

External consumers must re-implement the same visitor boilerplate on every project.

**Fix:** Provide at least one reference implementation (`TriviafreeVisitor`) in the
`visitor` crate that wraps another visitor and calls its methods only for non-trivia
children.

---

## D5 · No `serde` feature flag on any public type

**Layer**: cross-cutting · P4

`Token`, `CstTree`, `Diagnostic`, `SourceSpan`, `ResolvedProfile`, and other public
types have no `serde::Serialize` / `serde::Deserialize` derives. This blocks:

- Language server indexing caches (serialize parsed CST to disk).
- Test fixture generation (serialize a token stream to JSON for snapshot testing).
- Cross-process communication between a parse server and an editor plugin.

**Fix:** Add `[features] serde = ["dep:serde"]` to each crate's `Cargo.toml` and
gate `#[derive(Serialize, Deserialize)]` behind `#[cfg(feature = "serde")]`.

---

## D6 · `Tokenizer` is not `Clone` — cannot parallelise without rebuilding per thread

**Layer**: `rb_tokenizer` · P4
**File**: `crates/rb_tokenizer/src/tokenizers/tokenizer.rs`

`Tokenizer` stores scanners as `Vec<Box<dyn Scanner>>`. `dyn Scanner` does not require
`Clone`, so `Tokenizer` cannot derive or implement `Clone`. Calling `tokenize()` takes
`&self`, so parallel tokenization of multiple files requires either:

1. One `Tokenizer` behind a `Mutex` — serialises all tokenization, no parallelism.
2. One `Tokenizer` per thread — each must be independently constructed, which is
   expensive for a tokenizer with a large scanner list.

**Fix:** Add `clone_box()` to the `Scanner` trait (a standard vtable-clone pattern)
and derive `Clone` on `Tokenizer`:

```rust
pub trait Scanner: Send + Sync {
    // ...
    fn clone_box(&self) -> Box<dyn Scanner>;
}

impl Clone for Tokenizer {
    fn clone(&self) -> Self {
        Tokenizer {
            scanners: self.scanners.iter().map(|s| s.clone_box()).collect(),
        }
    }
}
```

---

## D7 · No `Display` / `to_sexpr()` on `CstTree`

**Layer**: `rb_parser` · P5
**File**: `crates/rb_parser/src/cst.rs`

There is no human-readable representation of a `CstTree`. Debugging a failed parse
requires writing a visitor from scratch or reading raw field dumps. S-expression
format (`(BinExpr (Ident "x") (Op "+") (Num "1"))`) is standard in parser ecosystems
for both debugging and snapshot testing.

**Fix:**

```rust
impl CstTree {
    pub fn to_sexpr(&self) -> String { ... }
}

impl std::fmt::Display for CstTree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_sexpr())
    }
}
```

---

## D8 · No fuzz testing infrastructure

**Layer**: cross-cutting · P4

Parsers are a primary target for misuse and bug-finding via fuzzing. There is no:

- `fuzz/` directory with `cargo-fuzz` targets.
- Integration with `honggfuzz` or `libFuzzer`.
- A fuzz target that feeds arbitrary bytes to `Tokenizer::tokenize()`.
- A fuzz target that feeds the tokenizer output to `CompiledParser::parse()`.

Without fuzz tests, malformed or adversarial input can cause panics (stack overflow
from left-recursive grammars, index-out-of-bounds in span arithmetic, etc.) that
would be caught quickly by even a short fuzzing run.

**Fix:** Create `fuzz/fuzz_targets/tokenizer.rs` and `fuzz/fuzz_targets/parser.rs`
targeting the tokenizer and parser pipelines. Run `cargo fuzz build` in CI.

---

## D9 · `#![deny(missing_docs)]` not set in any crate

**Layer**: cross-cutting · P5

None of the three crates enable `#![deny(missing_docs)]` or `#![warn(missing_docs)]`.
Public API surface grows without documentation. Once a public function is shipped
undocumented, adding documentation later is a lower-priority task that rarely happens.

**Fix:** Add `#![warn(missing_docs)]` to `lib.rs` of each crate now, fix the resulting
warnings iteratively, then promote to `#![deny(missing_docs)]` as a CI gate.

---

## D10 · No Criterion baseline tracking in CI

**Layer**: cross-cutting · P4
**File**: `crates/rb_tokenizer/benches/`, `crates/rb_parser/benches/`

Benchmarks exist but there is no:

- `cargo bench -- --save-baseline main` step in CI on the main branch.
- Comparison step on PRs that detects regressions above a threshold.
- Stored baseline artefact in the repo or CI cache.

The benchmark data collected during this audit (7–12× gap versus `serde_json`) cannot
serve as a meaningful baseline unless it is frozen and machine-comparable. Without CI
enforcement, performance regressions will go undetected between sessions.

**Fix:** Add a GitHub Actions workflow step:
```yaml
- name: Run benchmarks
  run: cargo bench --workspace -- --output-format=bencher | tee benchmark-results.txt
- name: Compare with baseline
  uses: benchmark-action/github-action-benchmark@v1
  with:
    tool: cargo
    output-file-path: benchmark-results.txt
    github-token: ${{ secrets.GITHUB_TOKEN }}
    alert-threshold: '120%'
    fail-on-alert: true
```

---

## A5 · `DiagnosticSeverity` deprecation alias has no migration path documented

**Layer**: `rb_common` · P5
**File**: `crates/rb_common/src/diagnostics.rs`

A `#[deprecated]` alias for an old severity name exists but the deprecation message
does not point to the replacement. Users who see the deprecation warning have no
indication of what to use instead.

**Fix:** Update the `#[deprecated]` attribute to include the `since` and `note`
parameters:

```rust
#[deprecated(since = "0.2.0", note = "Use `DiagnosticSeverity::Error` instead")]
pub type OldSeverityName = DiagnosticSeverity;
```
