# P0 — Correctness Bugs

**Priority**: P0 — Fix before any other work. These issues produce wrong output
or silently incorrect behaviour in the current implementation.

**Status**: All 12 issues resolved. 306 tests passing.

**Back to**: [Audit Index](README.md)

---

## A1 · PlainRenderer silently discards the error-line marker

**Status**: ✅ Fixed — `render.rs`: prefix `char` included in format string; `'>'` now emitted on error lines.

**Layer**: `rb_common`
**File**: `crates/rb_common/src/render.rs`
**Symptom**: Diagnostic snippets render all lines identically regardless of which line
contains the error.

### Root Cause

```rust
// render.rs — PlainRenderer::render
let prefix = if line.is_context { " " } else { ">" };
// ...
let _ = prefix;   // ← the marker is computed then immediately discarded
```

`prefix` is computed but assigned to `_`, so the `>` indicator that visually
distinguishes the primary error line from surrounding context lines is never emitted.
The output is a flat, undifferentiated block of source lines with no pointer to the
failure site.

### Impact

Every diagnostic produced through `PlainRenderer` is missing its most important visual
cue. Any user watching parser output sees error messages with no indication of which
line is wrong.

### Fix

Include `prefix` in the format string:

```rust
out.push_str(&format!(
    "{} {:pad$}{} | {}\n",
    prefix,        // ">" for error line, " " for context
    "",
    num_str,
    line.content,
    pad = pad
));
```

---

## A2 · `DefaultHasher` used for profile IDs — not stable across runs

**Status**: ✅ Fixed — `profiles.rs`: replaced `DefaultHasher` with deterministic FNV-64 (`fnv64()` helper).

**Layer**: `rb_common`
**File**: `crates/rb_common/src/profiles.rs`
**Symptom**: Grammar caches keyed by `ResolvedProfileId` silently produce cache misses
or wrong hits across different processes or Rust versions.

### Root Cause

```rust
// profiles.rs — ResolvedProfileId::compute
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
let mut h = DefaultHasher::new();
// ...
ResolvedProfileId(h.finish())
```

`DefaultHasher` is explicitly documented in the Rust standard library as not providing
any stability guarantees: the hash produced for the same input may differ between
program runs, between Rust versions, and between platforms. Any cache keyed by this ID
will behave non-deterministically.

### Impact

`ResolvedProfileId` appears in the spec as a stable, deterministic key for compiled
grammar caching. Using `DefaultHasher` defeats that guarantee entirely.

### Fix

Replace with a deterministic algorithm — FNV-64 is zero-dependency and appropriate
for this use case:

```rust
// Deterministic FNV-64 — constant across runs, platforms, and Rust versions
fn fnv64(bytes: &[u8]) -> u64 {
    const OFFSET: u64 = 14695981039346656037;
    const PRIME:  u64 = 1099511628211;
    bytes.iter().fold(OFFSET, |hash, &b| hash.wrapping_mul(PRIME) ^ (b as u64))
}

pub fn compute(language: &str, version: LanguageVersion, mode: ProfileMode, features: &[FeatureFlag]) -> Self {
    let mut buf = String::new();
    buf.push_str(language);
    buf.push('\x00');
    buf.push_str(&version.to_string());
    buf.push('\x00');
    buf.push_str(&mode.to_string());
    for f in features {
        buf.push('\x00');
        buf.push_str(f.0);
    }
    ResolvedProfileId(fnv64(buf.as_bytes()))
}
```

---

## A3 · `SourceSpan::merge` of two UNKNOWN spans returns a valid-looking span

**Status**: ✅ Fixed — `spans.rs`: UNKNOWN identity guards added to `merge()`; UNKNOWN merged with any span returns the non-UNKNOWN span.

**Layer**: `rb_common`
**File**: `crates/rb_common/src/spans.rs`
**Symptom**: Merging two `SourceSpan::UNKNOWN` values (both zeros, both `SourceId(0)`)
returns a non-UNKNOWN span with byte offsets of zero that is visually indistinguishable
from a real span covering the first byte of a file.

### Root Cause

`SourceSpan::merge` checks only that source IDs match. `SourceId(0)` is the sentinel
value for UNKNOWN but is also a valid-looking source ID. Two UNKNOWN spans will pass
the equality check and be merged to a zero-offset span.

### Impact

Diagnostics produced from tokens without real spans will have their locations silently
conflated with byte offset 0 in the first file. Error messages that say "line 1,
column 1" when the real location is unknown are harder to debug than an explicit
"location unknown".

### Fix

Add an UNKNOWN guard to `merge`:

```rust
pub fn merge(self, other: SourceSpan) -> Option<SourceSpan> {
    if self == SourceSpan::UNKNOWN { return Some(other); }
    if other == SourceSpan::UNKNOWN { return Some(self); }
    if self.source_id != other.source_id { return None; }
    // ... existing logic
}
```

---

## B1 · `Scanner::scan_with_context` derives `consumed_len` from `value.len()`

**Status**: ✅ Fixed — `scanner.rs`: added `ScanMatch::verbatim(token)` and `ScanMatch::with_consumed(token, consumed_len)` constructors; trait doc updated to make override obligation explicit.

**Layer**: `rb_tokenizer`
**File**: `crates/rb_tokenizer/src/scanners/scanner.rs`
**Symptom**: The tokenizer loop miscalculates the cursor advance when a scanner returns
a value whose byte length differs from the input it consumed.

### Root Cause

```rust
// scanner.rs — Scanner::scan_with_context default impl
fn scan_with_context(&self, input: &str) -> Result<Option<ScanMatch>, TokenizationError> {
    self.scan(input).map(|result| {
        result.map(|token| ScanMatch {
            consumed_len: token.value.len(),   // ← derived from value, not from consumed input
            token,
        })
    })
}
```

Scanners that strip delimiters (e.g. `BlockScanner` with `include_delimiters: false`)
return a `value` that is shorter than the actual input consumed. The tokenizer loop
uses `consumed_len` to advance its byte cursor and call `advance_cursor` for
line/column tracking. When the value is shorter than the consumed input, the cursor
stops short, causing the next scanner attempt to start inside the already-consumed
token.

### Concrete example

For a block comment `/* foo */` with `include_delimiters: false`, `value` is `" foo "`
(7 bytes) but the consumed input is 9 bytes. The cursor advances 7 bytes, leaving `*/`
as the start of the next scan attempt — a tokenizer loop infinite-error cascade.

### Impact

Any scanner that transforms its captured text (stripping delimiters, collapsing
escapes, trimming quotes) will misalign the tokenizer. Block comments, block strings,
and raw string literals are all affected.

### Fix

`Scanner` implementations must override `scan_with_context` explicitly when their
`value` differs from the consumed input. Add a `consumed_len` field to the return
path of affected scanners, and update the `Scanner` trait documentation to make this
obligation clear:

```rust
/// Implementations that return a `value` shorter than the consumed input
/// MUST override `scan_with_context` and set `consumed_len` to the actual
/// number of bytes consumed from `input`, not the length of the returned value.
fn scan_with_context(&self, input: &str) -> Result<Option<ScanMatch>, TokenizationError> { ... }
```

Provide a `ScanMatch::new(token, consumed_len: usize)` constructor that forces the
caller to be explicit.

---

## B2 · `ContextualScanner` silently invisible when `tokenize()` is called

**Status**: ✅ Fixed — `tokenizer.rs`: added `has_contextual_scanners()` method; `#[cfg(debug_assertions)]` panic guard at start of `tokenize()` if contextual scanners are registered.

**Layer**: `rb_tokenizer`
**File**: `crates/rb_tokenizer/src/scanners/scanner_types.rs`
**Symptom**: A `ContextualScanner` registered via `add_contextual_scanner` produces no
output and no error when the caller uses `tokenize()` instead of `tokenize_contextual()`.

### Root Cause

```rust
// scanner_types.rs — ScannerType::Contextual path in scan_with_context
ScannerType::Contextual(_) => Ok(None),   // silently returns no match
```

The standard tokenization path returns `Ok(None)` for every contextual scanner. There
is no warning to the caller that registering a contextual scanner and then calling the
standard `tokenize()` is a no-op for those scanners.

### Impact

A grammar author who builds a context-sensitive tokenizer (e.g. for Python indentation,
JSX, or template literals) using `add_contextual_scanner` and then calls `tokenize()`
will see their contextual tokens silently dropped from the output with no diagnostic.
This is an extremely confusing failure mode.

### Fix

Two changes are needed:

1. In `Tokenizer::tokenize()`, after building the scanner list, emit a
   `TokenizationError` or panic in debug mode if any registered scanner is
   `ScannerType::Contextual`:

```rust
// In Tokenizer::tokenize() — guard at entry
#[cfg(debug_assertions)]
if self.scanners.iter().any(|s| matches!(s, ScannerType::Contextual(_))) {
    panic!(
        "Tokenizer has contextual scanners registered. \
         Use tokenize_contextual() instead of tokenize()."
    );
}
```

2. Add a `Tokenizer::has_contextual_scanners() -> bool` method so callers can
   select the right tokenization path programmatically.

---

## B3 · `TokenizationError` carries no source position when errors are returned

**Status**: ✅ Fixed — `error.rs`: added `WithSpan { error: Box<TokenizationError>, span: SourceSpan }` variant plus `span()`, `inner()`, `at()` helpers; tokenizer loop wraps all errors with `.at(err_span)` at point of push.

**Layer**: `rb_tokenizer`
**File**: `crates/rb_tokenizer/src/tokenizers/tokenizer.rs`
**Symptom**: Errors returned in `Err(Vec<TokenizationError>)` from `tokenize()` have
no byte position, line, or column information.

### Root Cause

When a scanner returns `Err(e)`, the error is pushed into the error Vec before the
cursor position has been computed for that position:

```rust
Err(e) => {
    errors.push(e);   // ← pushed before position is known
    // ...
}
```

`TokenizationError` has no span field; position must be injected after construction.
Since errors are pushed immediately from scanner results and scanners themselves do not
receive position context, the errors arrive without location.

### Impact

The `DiagnosticsContext` integration spec (Phase 1) requires all tokenizer errors to
carry at minimum a byte offset. Callers receiving a `Vec<TokenizationError>` for a
500-line file have no way to tell the user where the problem is.

### Fix

Add `span: Option<SourceSpan>` to `TokenizationError`. In the tokenizer loop, enrich
errors with the position computed from `current_line`, `current_column`, and `start`
before pushing:

```rust
Err(mut e) => {
    e.span = Some(SourceSpan {
        source_id: self.source_id,
        start: SourcePosition { byte_offset: start, line: current_line - 1, column: current_column - 1 },
        end: SourcePosition { byte_offset: start, line: current_line - 1, column: current_column - 1 },
    });
    errors.push(e);
    // ...
}
```

---

## C1 · `CstTree::walk_node` clones every node's child list during traversal

**Status**: ✅ Fixed — `cst.rs`: now collects `Vec<NodeOrToken>` (Copy IDs) before visitor callbacks, removing `Vec<CstNodeChild>` clone per node.

**Layer**: `rb_parser`
**File**: `crates/rb_parser/src/cst.rs`
**Symptom**: A full tree walk allocates one `Vec<CstNodeChild>` per visited node.

### Root Cause

```rust
// cst.rs — CstTree::walk_node
pub fn walk_node(&self, node_id: SyntaxNodeId, visitor: &mut dyn TreeVisitor) {
    let node = self.node(node_id);
    visitor.visit_node_enter(node, self);
    for child in &node.children.clone() {   // ← clones the entire Vec
        // ...
    }
    let node = self.node(node_id);
    visitor.visit_node_exit(node, self);
}
```

The `clone()` was added to satisfy the borrow checker: `visit_node_enter` takes
`(&CstNode, &CstTree)` where the node borrows from `self`, and iterating children
separately would create a second borrow. The clone sidesteps this at the cost of one
heap allocation per node.

### Impact

A visitor traversal of a tree with 10,000 nodes allocates 10,000 temporary Vecs.
Visitor traversal — the most common post-parse operation in IDE tooling and AST
lowering pipelines — becomes an O(N) allocation waterfall that defeats the arena-based
build design.

### Fix

Change `walk_node` to collect child IDs (which are `Copy` integers) before calling
`visit_node_enter`, avoiding any borrow conflict:

```rust
pub fn walk_node(&self, node_id: SyntaxNodeId, visitor: &mut dyn TreeVisitor) {
    // Snapshot child IDs before calling visitor — u32 copies, no allocation
    let children: Vec<NodeOrToken> = self.node(node_id).children
        .iter()
        .map(|c| c.child)
        .collect();

    visitor.visit_node_enter(self.node(node_id), self);
    for child in children {
        match child {
            NodeOrToken::Node(id) => self.walk_node(id, visitor),
            NodeOrToken::Token(id) => {
                let tok = self.token(id);
                if tok.is_trivia { visitor.visit_trivia_token(tok, self); }
                else             { visitor.visit_token(tok, self); }
            }
        }
    }
    visitor.visit_node_exit(self.node(node_id), self);
}
```

This reduces the per-traversal allocation from one `Vec<CstNodeChild>` per node to one
`Vec<NodeOrToken>` per node — still allocating, but smaller elements. The deeper fix is
storing children in a flat arena on `CstTree` (see P1 — C8) so `walk_node` can slice
directly into `tree.children_store` with no allocation.

---

## C2 · `GrammarBuilder::rule` silently overwrites duplicate rule IDs

**Status**: ✅ Fixed — `combinator.rs`: `GrammarBuilder::rule` now panics immediately if a rule ID is registered twice.

**Layer**: `rb_parser`
**File**: `crates/rb_parser/src/grammar/combinator.rs`
**Symptom**: Calling `.rule(MyRule::Foo, a)` and then `.rule(MyRule::Foo, b)` silently
discards `a` and uses `b`. No error is produced. `GrammarError::DuplicateRule` is
defined but never emitted.

### Root Cause

```rust
// combinator.rs — GrammarBuilder::rule
pub fn rule(mut self, rule_id: R, rule: impl GrammarRule<R>) -> Self {
    let key = format!("{:?}", rule_id);
    self.rules.insert(key, (rule_id, rule.into()));   // IndexMap::insert overwrites silently
    self
}
```

`IndexMap::insert` returns the previous value if the key existed. The return value is
discarded. The compile step never checks for this.

### Impact

A grammar author who accidentally registers the same rule twice (a common mistake during
prototyping or refactoring) gets a silently broken grammar where one rule definition is
lost. The grammar will parse incorrectly and the author has no feedback.

### Fix

```rust
pub fn rule(mut self, rule_id: R, rule: impl GrammarRule<R>) -> Self {
    let key = format!("{:?}", rule_id);
    if self.rules.contains_key(&key) {
        panic!(
            "Grammar rule {:?} registered twice. \
             Use Grammar::compile() error handling or ensure each rule is registered once.",
            rule_id
        );
    }
    self.rules.insert(key, (rule_id, rule.into()));
    self
}
```

Or alternatively surface it through `Grammar::compile()` as `Err(GrammarError::DuplicateRule { ... })`.
The panic approach fails fast at development time (which is appropriate for a rule
registration mistake); the `compile()` error approach is cleaner for dynamic grammar
construction.

---

## C4 · `ParseContext::recovery` is never consulted by the parse engine

**Status**: ✅ Fixed — `engine.rs`: removed the dead `recovery` field from `ParseContext` and its constructor parameter; all three `ParseContext::new()` call sites in `lib.rs` updated accordingly.

**Layer**: `rb_parser`
**File**: `crates/rb_parser/src/engine.rs`, `crates/rb_parser/src/grammar/combinator.rs`
**Symptom**: Setting `RecoveryConfig` on a `CompiledParser` has no effect on
recovery behaviour.

### Root Cause

```rust
// engine.rs — ParseContext
pub(crate) recovery: crate::profiles::RecoveryConfig,
#[allow(dead_code)]  // ← noted as dead
```

The `recovery` field is stored in `ParseContext` but never read. The `eval` function
receives `max_recovery_steps` as a separate function parameter:

```rust
// lib.rs — run_building
let _ = eval(start_expr, &mut parse_ctx, &self.grammar, &mut strategy, None,
             &mut recovery_steps, recovery.max_recovery_steps);
                                         //  ↑ passed as a parameter, ignores parse_ctx.recovery
```

`recovery.max_recovery_steps` is extracted from the `RecoveryConfig` field on
`CompiledParser` and passed as a raw `usize` to `eval`. The `ParseContext::recovery`
field is a separate clone that is never used.

### Impact

Callers who configure `CompiledParser` recovery settings expect them to be honoured.
Because the `ParseContext` copy is dead, any configuration beyond `max_recovery_steps`
(such as `warn_on_limit`) has no effect.

### Fix

Remove the `recovery` field from `ParseContext` (it's already dead). Access the
`RecoveryConfig` from `CompiledParser` directly when constructing the `ParseContext`,
and pass all relevant fields as parameters to `eval` — or refactor `eval` to receive
a `&RecoveryConfig` reference:

```rust
fn eval<R, S>(
    expr: &RuleExpr<R>,
    ctx: &mut ParseContext,
    grammar: &CompiledGrammar<R>,
    strategy: &mut S,
    field: Option<&'static str>,
    recovery_steps: &mut usize,
    recovery: &RecoveryConfig,   // ← full config, not just one field
) -> ParseOutcome<()>
```

---

## C5 · Unclosed error nodes get zero-width spans that cannot be identified as synthetic

**Status**: ✅ Fixed — `cst.rs`: added `is_error_recovery: bool` field to `CstNode` (default `false`); `strategy.rs`: nodes synthesized in `finish()` for unclosed stack entries have `is_error_recovery: true`.

**Layer**: `rb_parser`
**File**: `crates/rb_parser/src/strategy.rs`
**Symptom**: When the parser stack has unclosed nodes after a parse error, synthesized
close events produce nodes with `span.start == span.end`, which is the same as a
legitimate empty node or an insertion-point span.

### Root Cause

```rust
// strategy.rs — CstBuildingStrategy::finish
let span = SourceSpan::new(self.source_id, start, start);  // zero-width, indistinguishable
```

### Impact

Tools consuming the CST (linters, formatters, AST lowerers) cannot distinguish a
node that was legitimately empty from a node that was synthesized during error recovery.
This makes precise error recovery analysis and partial-parse tools unreliable.

### Fix

Add an `is_error_recovery: bool` flag to `CstNode`, set to `true` for synthesized
nodes produced during `finish()`. Alternatively, use a dedicated `SyntaxKind` prefix
convention (e.g. `"error::..."`) for error-recovery nodes to make them filterable
without a new field.

---

## C9 · `CstNode::direct_tokens()` includes trivia; `CstTree::tokens_of()` does not

**Status**: ✅ Fixed — `cst.rs`: `direct_tokens()` doc updated with clear "includes trivia" warning; new `direct_semantic_tokens(tree)` method added that is consistent with `CstTree::tokens_of()`.

**Layer**: `rb_parser`
**File**: `crates/rb_parser/src/cst.rs`
**Symptom**: Calling `node.direct_tokens()` returns all tokens including whitespace and
comments. Calling `tree.tokens_of(node_id)` returns only semantic (non-trivia) tokens.
The two methods appear to do the same thing but behave differently.

### Root Cause

```rust
// cst.rs — CstNode::direct_tokens — includes trivia
pub fn direct_tokens(&self) -> impl Iterator<Item = SyntaxTokenId> + '_ {
    self.children.iter().filter_map(|c| c.child.as_token())
    // no trivia filter
}

// cst.rs — CstTree::tokens_of — excludes trivia
pub fn tokens_of(&self, node_id: SyntaxNodeId) -> impl Iterator<Item = &CstToken> + '_ {
    // ...
    .filter(|t| !t.is_trivia)   // trivia filtered out
}
```

### Impact

Calling `node.direct_tokens()` on a node with whitespace children produces trivia
tokens in the result without any indication. Code that iterates over supposedly
semantic tokens (e.g. extracting identifiers) will inadvertently include whitespace.

### Fix

Either:
1. Rename to make intent explicit: `direct_tokens_including_trivia()` vs `direct_semantic_tokens()`.
2. Make `CstNode::direct_tokens()` consistent with `CstTree::tokens_of()` — return
   only semantic tokens — and add a separate `CstNode::all_tokens_including_trivia()`
   for the rare case where trivia is intentionally needed.

---

## D1 · `visitor` crate referenced in workspace structure does not exist on disk

**Status**: ✅ Fixed — created `crates/visitor/` with `Cargo.toml` and `src/lib.rs` (re-exports `TreeVisitor`, `DepthFirstWalker`, `WalkOrder` from `rb_parser::visitors`); added to workspace `[members]` in root `Cargo.toml`.

**Layer**: workspace
**File**: `docs/` and previous workspace structure references
**Symptom**: The `crates/visitor/` directory is referenced in README/workspace docs
but does not exist.

### Fix

Either create the `visitor` crate skeleton with `Cargo.toml` and a minimal `src/lib.rs`,
or remove all references to it from the workspace description and docs until it is
planned for implementation.
