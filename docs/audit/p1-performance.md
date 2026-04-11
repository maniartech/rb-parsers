# P1 — Performance Issues

**Priority**: P1 — These issues directly cause the 7–12× throughput gap versus
`serde_json`. Address in the order listed: **B4** and **B5** account for the majority
of the gap and should be fixed first.

**Back to**: [Audit Index](README.md)

> **LSP / Production Language note**: Current pipeline throughput is ~2.3 MB/s.
> A responsive language tool needs ≥ 20 MB/s; an IDE language server needs ≥ 50 MB/s.
> B4 + B5 + B7 are the critical path to closing that gap.
> See [README — Performance Blockers](README.md#full-programming-language--performance-blockers-p1).

**Benchmark baseline** (collected using `cargo bench --workspace`):

| Input   | rb_pipeline | serde_json | Ratio |
|---------|-------------|------------|-------|
| small   | 13.4 µs     | 1.09 µs    | 12.3× |
| medium  | 274 µs      | 37 µs      | 7.4×  |
| large   | 2.37 ms     | 246 µs     | 9.6×  |

---

## B4 · `value: String` in `Token` — heap allocation per token

**Layer**: `rb_tokenizer`
**File**: `crates/rb_tokenizer/src/tokens/token.rs`
**Expected impact**: 40–60% reduction in bench time when combined with B5.

### Root Cause

```rust
pub struct Token {
    pub token_type:     String,
    pub token_sub_type: Option<String>,
    pub value:          String,   // ← owned String per token
    // ...
}
```

Every scanner match creates at minimum one heap allocation for the `value` field.
For a medium JSON file (50 000 tokens), this is 50 000 `malloc` / `free` cycles inside
the hot tokenization loop.

### Fix

Replace `value` with `Cow<'src, str>`:

```rust
pub struct Token<'src> {
    pub token_type:     &'static str,     // always a literal — borrow
    pub token_sub_type: Option<&'static str>,
    pub value:          Cow<'src, str>,   // borrow from input; only allocate for
                                          // escape-decoded or synthesised tokens
}
```

Scanner types that merely slice the input (identifiers, numbers, punctuation, block
comments with `include_delimiters: true`) return `Cow::Borrowed(&input[start..end])`.
Only escape-processing scanners (quoted strings with `\n`, `\uXXXX`, etc.) allocate a
new `String` via `Cow::Owned`.

This is a breaking change to the `Token` struct. It requires adding a lifetime
parameter to `Tokenizer` and `TokenStream`. See related spec `rb_tokenizer-token-upgrade.md`.

---

## B5 · `advance_cursor` walks every byte for line/column tracking — O(N) per token

**Layer**: `rb_tokenizer`
**File**: `crates/rb_tokenizer/src/tokenizers/tokenizer.rs`
**Expected impact**: 25–35% reduction after B4.

### Root Cause

```rust
fn advance_cursor(value: &str, line: &mut usize, col: &mut usize) {
    for ch in value.chars() {   // ← iterates every character every token
        if ch == '\n' {
            *line += 1;
            *col = 1;
        } else {
            *col += 1;
        }
    }
}
```

For every token matched, the tokenizer iterates every byte/char of the token value to
update `(line, col)`. This means line tracking costs O(characters in token). A 10 KB
JSON string literal requires 10 KB of character-by-character iteration solely for
position accounting.

### Fix

Track only the **byte offset** on the hot path. Compute `(line, col)` lazily — only
when a `SourceSpan` is about to be attached to a token or a diagnostic.

```rust
// Lazy position computation using a SourceMap built once per file
pub struct SourceMap {
    /// Byte offset of each newline character (0-indexed)
    line_starts: Vec<u32>,
}

impl SourceMap {
    pub fn from_source(source: &str) -> Self {
        use memchr::memchr_iter;
        let line_starts = std::iter::once(0u32)
            .chain(memchr_iter(b'\n', source.as_bytes()).map(|i| (i + 1) as u32))
            .collect();
        Self { line_starts }
    }

    pub fn position_of(&self, byte_offset: usize) -> (u32, u32) {
        let line = self.line_starts.partition_point(|&s| s <= byte_offset as u32) - 1;
        let col  = byte_offset - self.line_starts[line] as usize;
        (line as u32, col as u32)
    }
}
```

`memchr::memchr_iter` is SIMD-accelerated on x86/aarch64 and builds the newline table
in one pass over the source bytes. The resulting binary search in `position_of` is
O(log lines) — effectively free for the diagnostic path.

`memchr` is already an indirect dependency through `aho-corasick`. Adding it as a
direct dependency has zero extra cost.

---

## B6 · `keyword.clone()` / `operator.clone()` in scanner hot paths

**Layer**: `rb_tokenizer`
**File**: `crates/rb_tokenizer/src/scanners/keyword_scanner.rs`,
         `crates/rb_tokenizer/src/scanners/operator_scanner.rs`
**Expected impact**: ~5%.

### Root Cause

```rust
// KeywordScanner::scan
Ok(Some(PartialToken {
    token_type: self.token_type.clone(),    // String clone
    value: keyword.clone(),                 // String clone of a &str already owned
    // ...
}))

// OperatorScanner::scan  — same pattern
Ok(Some(PartialToken {
    token_type: self.token_type.clone(),
    value: operator.clone(),
    // ...
}))
```

Keywords and operators are compile-time constants (`&'static str`). Storing them as
`String` in scanner state and cloning on every match is unnecessary.

### Fix

After the B4 token lifetime change, scanners can return `Cow::Borrowed`:

```rust
// With Token<'src> carrying Cow<'src, str>
value: Cow::Borrowed(keyword),    // zero allocation — borrows the &'static str
```

No code path needs a keyword or operator `value` to outlive the corresponding input
slice, so `Borrowed` is correct for both.

---

## B7 · Linear O(scanners) dispatch per input position

**Layer**: `rb_tokenizer`
**File**: `crates/rb_tokenizer/src/tokenizers/tokenizer.rs`
**Expected impact**: 15–30% in grammars with more than ~8 scanners.

### Root Cause

```rust
// tokenizer.rs — main tokenize loop
for scanner in &self.scanners {
    match scanner.scan_with_context(remaining) {
        Ok(Some(m)) => { /* use m */ break; }
        Ok(None)    => continue,
        Err(e)      => { errors.push(e); continue; }
    }
}
```

Every byte position tries every scanner in registration order. A tokenizer with 20
scanners performs 20 virtual dispatch calls per position on average (more for positions
where no scanner matches, fewer where the first matches). For a 50-token file this is
a minor constant; for a 100 000-token file it becomes the dominant cost.

### Fix

Build a first-byte dispatch table at `Tokenizer` construction time:

```rust
pub struct Tokenizer {
    scanners: Vec<Box<dyn Scanner>>,
    /// scanners[i] should be tried when the next byte matches first_byte_table[byte]
    first_byte_table: [SmallVec<[u8; 4]>; 256],
}
```

On construction, call a `first_bytes() -> Option<&[u8]>` hint method on each
`Scanner`:
- Scanners that always start with a specific byte (punctuation, string delimiters,
  operators) return `Some(&[b'"'])`.
- Scanners that can start with any byte (identifiers for non-ASCII-initial langs,
  catch-all error recovery) return `None` and remain in a fallback list.

In the hot loop, only try `first_byte_table[input.as_bytes()[0]]` scanners, plus the
fallback list.

---

## B8 · 12-arm `match` on `ScannerType` executed per position

**Layer**: `rb_tokenizer`
**File**: `crates/rb_tokenizer/src/scanners/scanner_types.rs`
**Expected impact**: Small but measured when branch prediction misses are high.

### Root Cause

```rust
pub enum ScannerType {
    Keyword(KeywordScanner),
    Operator(OperatorScanner),
    Block(BlockScanner),
    EOL(EOLScanner),
    Regex(RegexScanner),
    // ... 12 arms
    Contextual(Box<dyn Scanner>),
}

impl Scanner for ScannerType {
    fn scan_with_context(&self, input: &str) -> ... {
        match self {
            Self::Keyword(s)  => s.scan_with_context(input),
            Self::Operator(s) => s.scan_with_context(input),
            // ... 12 arms
        }
    }
}
```

The 12-arm match is evaluated at every scanner dispatch. Modern branch prediction
handles well-predicted matches fine, but a mixed scanner list (common in real grammars)
experiences mispredictions.

### Fix

This is superseded by the B7 first-byte table fix. If scanners are pre-grouped by
first-byte, each dispatch sub-list is small enough that branch prediction becomes
nearly perfect. No separate fix needed once B7 is implemented.

---

## A8 · `SpanLabel::message: Option<String>` allocates for static-string messages

**Layer**: `rb_common`
**File**: `crates/rb_common/src/spans.rs`
**Expected impact**: Minor — relevant only in diagnostic-heavy parse sessions.

### Root Cause

```rust
pub struct SpanLabel {
    pub span:    SourceSpan,
    pub message: Option<String>,   // ← allocates even for a &'static str label
    pub style:   LabelStyle,
}
```

Virtually all `SpanLabel` messages in practice are string literals (e.g. `"expected ';' here"`).
Using `Option<String>` forces an allocation for every label regardless.

### Fix

```rust
pub struct SpanLabel {
    pub span:    SourceSpan,
    pub message: Option<Cow<'static, str>>,   // borrows for literals, owns for dynamic
    pub style:   LabelStyle,
}
```

Construction from a string literal becomes `Some(Cow::Borrowed("expected ';' here"))`.
Dynamic messages remain `Some(Cow::Owned(format!(...)))`. This is a minor source
compatibility change.

---

## C6 · `format!("{:?}", rule_id)` called in `resolve_refs` — repeated allocation during compilation

**Layer**: `rb_parser`
**File**: `crates/rb_parser/src/grammar/combinator.rs`
**Expected impact**: Compilation-time only (not parse-time).

### Root Cause

```rust
fn resolve_refs<R>(rules: &RulesMap<R>, expr: &RuleExpr<R>) -> RuleExpr<R> {
    match expr {
        RuleExpr::Ref(rule_id) => {
            let key = format!("{:?}", rule_id);   // ← called per Ref node in every rule
            // ...
        }
        // ... recursion through all sub-expressions
    }
}
```

`resolve_refs` walks the entire grammar expression tree recursively. Every `Ref` node
causes a `format!("{:?}", ...)` allocation. A grammar with 50 rules and 200 cross-references
will call `format!` 200+ times during compilation.

### Fix

Pre-build a `HashMap<String, usize>` of rule keys at the start of `Grammar::compile()`,
and pass it as parameter rather than re-calling `format!` per node:

```rust
let key_to_idx: HashMap<String, usize> = rules
    .iter()
    .enumerate()
    .map(|(i, (k, _))| (k.clone(), i))
    .collect();
```

Then `resolve_refs` receives `&HashMap<String, usize>` instead of `&RulesMap<R>`,
eliminating all `format!` calls in the recursive traversal.

---

## C8 · `CstNode::children: Vec<CstNodeChild>` — per-node allocation despite arena build

**Layer**: `rb_parser`
**File**: `crates/rb_parser/src/cst.rs`, `crates/rb_parser/src/strategy.rs`
**Expected impact**: 20–30% reduction in post-parse traversal allocations.

### Root Cause

The `CstBuildingStrategy` uses a node stack to build the tree and emits `NodeEnd` events
that call `Vec::drain` to move children to the new node:

```rust
// strategy.rs — on NodeEnd
let mut children = self.pending_children.drain(frame.children_start..).collect::<Vec<_>>();
let node = CstNode { children, /* ... */ };   // ← new Vec<CstNodeChild> per node
```

Every `CstNode` owns its children in a freshly allocated `Vec`. While the build phase
itself is arena-efficient (the `pending_children` stack avoids many allocations), the
final `CstTree` contains one `Vec` per non-leaf node — anywhere from hundreds to
hundreds of thousands of heap allocations for the lifetime of the tree.

### Fix

Flatten children into a single `Vec<CstNodeChild>` owned by `CstTree` and store
`(start: u32, len: u32)` on each `CstNode`:

```rust
pub struct CstTree {
    nodes:          Vec<CstNode>,
    tokens:         Vec<CstToken>,
    children_store: Vec<CstNodeChild>,   // flat arena — shared by all nodes
}

pub struct CstNode {
    pub kind:         SyntaxKind,
    pub span:         SourceSpan,
    pub children_start: u32,
    pub children_len:   u16,   // unlikely to exceed 65535 for a single node
}
```

`walk_node` and `tokens_of` slice `&tree.children_store[start..start+len]` with no
allocation. This also eliminates the C1 `clone()` fix because the borrow is now into
`tree.children_store`, which is independent of the node under the visitor's mutable
reference.

---

## E1 · Trivia token types not configurable — trivia always flows into the parser

**Layer**: `rb_tokenizer`
**File**: `crates/rb_tokenizer/src/tokenizers/tokenizer.rs`
**Expected impact**: 5–15% on source-code inputs with heavy whitespace/comments; zero impact on JSON-style inputs.

### Root Cause

`TokenizerConfig` has a single `tokenize_whitespace: bool` flag (default `false`) that
only suppresses `WhitespaceScanner` output. All other token types — line comments,
block comments, shebangs, BOMs — flow unconditionally into `Vec<Token>` regardless of
whether the grammar needs them.

The parser marks tokens as `is_trivia: true` in `ParseEvent::Token` and `CstToken`,
meaning trivia tokens traverse the full pipeline (tokenizer → `Vec<Token>` → parse
engine → `CstBuildingStrategy::on_event` → `CstNode::children`) even when they carry
no semantic content and exist solely for formatter round-tripping.

For a minified source file the cost is zero. For well-commented, indented source code
(the common case for a language server) trivia tokens can outnumber semantic tokens 3:1.
Every one of those tokens incurs allocation (B4), position tracking (B5), and CST
insertion overhead (C8) regardless of whether the consumer will ever look at them.

### Fix

Extend `TokenizerConfig` with a set of token types to be silently discarded after
matching — never emitted to the output stream:

```rust
pub struct TokenizerConfig {
    pub tokenize_whitespace: bool,
    /// Token types in this set are matched (so they are consumed from the input)
    /// but never emitted. The consumer never sees them.
    pub drop_token_types: std::collections::HashSet<&'static str>,
    // ...
}
```

Grammar authors declare trivia for their language once, at tokenizer construction:

```rust
config.drop_token_types.insert("WHITESPACE");
config.drop_token_types.insert("LINE_COMMENT");
config.drop_token_types.insert("BLOCK_COMMENT");
```

The hot loop drops the match immediately after `consumed_len` is known, before
allocating a `Token`:

```rust
// tokenizer hot loop — after a match is found
if self.config.drop_token_types.contains(match_.token.token_type) {
    byte_offset += match_.consumed_len;
    continue;   // advance cursor, emit nothing
}
// ... rest of token construction
```

A complementary `TriviaMode` enum can provide higher-level control for consumers
that do need trivia (formatters, refactoring tools):

```rust
pub enum TriviaMode {
    /// Trivia tokens are dropped at the tokenizer; the CST has no trivia nodes.
    /// Best for compilers and semantic-only tools.
    Drop,
    /// Trivia tokens flow through the pipeline; the CST marks them `is_trivia: true`.
    /// Best for formatters and IDE round-trip tools. (Current default behaviour.)
    Attach,
}
```

### Benchmark impact

This does not move the existing `vs_serde_json` benchmark (JSON has no whitespace in
the bench fixtures). To measure the gain, add a `lex_source_code_with_comments`
benchmark that feeds a realistic indented source file. Expected: ~10% reduction in
`Vec<Token>` size and proportional reduction in parse-engine work for typical
prose-style source code.

### Dependency

None. This change is independent of B4 and can be implemented first. However, the
full benefit is compounded when combined with B4 (Cow tokens), since the tokens that
are not dropped become cheaper to construct.

---

## E2 · Full `Vec<Token>` materialization — parser cannot consume tokens lazily

**Layer**: `rb_tokenizer` / `rb_parser`
**File**: `crates/rb_tokenizer/src/tokenizers/tokenizer.rs`, `crates/rb_parser/src/lib.rs`
**Expected impact**: 10–20% additional throughput improvement on top of B4+B5 for large files; significant memory reduction.

### Root Cause

`Tokenizer::tokenize(input: &str) -> Vec<Token>` always allocates and fills the
complete token vector before returning. `CompiledParser::parse_tree` receives
`&[Token]` and holds a cursor index into it.

For a 10 MB source file this means the entire token array must be heap-allocated and
populated before the first grammar rule fires. Peak memory usage is approximately
`n_tokens * size_of::<Token>()` (currently ~96 bytes per token = ~100 MB for 1 million
tokens) in addition to the source string itself.

A PEG parser needs lookahead to backtrack, but in practice most grammars only look
ahead 1–3 tokens at any committed parse point. The entire token array is kept alive to
support the worst-case backtrack depth even though committed tokens are never
re-examined.

### Fix

Define a `TokenSource` pull trait and a `BufferedTokenSource` adapter:

```rust
/// Pull-based token source consumed by the parse engine.
pub trait TokenSource {
    /// Returns the token at `offset` positions ahead of the current position.
    /// `offset = 0` is the current token.
    fn peek(&self, offset: usize) -> Option<&Token>;
    /// Advance past the current token.
    fn advance(&mut self);
    /// Current byte position (for diagnostics).
    fn current_byte_offset(&self) -> usize;
}

/// Wraps any `Iterator<Item = Token>` and maintains a sliding lookahead window.
pub struct BufferedTokenSource<I: Iterator<Item = Token>> {
    iter:   I,
    buffer: std::collections::VecDeque<Token>,
}
```

`ParseContext` is migrated from `(tokens: &[Token], pos: usize)` to
`token_source: &mut dyn TokenSource`. Once `eval()` commits past a token (i.e. no
live backtrack frame can reach it), `BufferedTokenSource` discards it from the
`VecDeque`, keeping memory proportional to the current backtrack window rather than
the file size.

The existing `parse_tree(&[Token])` API is preserved as a convenience wrapper that
constructs a `SliceTokenSource`:

```rust
pub struct SliceTokenSource<'a> {
    tokens: &'a [Token],
    pos: usize,
}
impl<'a> TokenSource for SliceTokenSource<'a> {
    fn peek(&self, offset: usize) -> Option<&Token> { self.tokens.get(self.pos + offset) }
    fn advance(&mut self) { self.pos += 1; }
    fn current_byte_offset(&self) -> usize { /* from span */ 0 }
}
```

This makes the API change non-breaking for existing callers while enabling streaming
use for new ones.

### Bounded backtrack window

Once C15 (FIRST set analysis) is implemented, the maximum backtrack depth per grammar
position becomes statically known. `BufferedTokenSource` can expose a
`set_window_limit(n: usize)` method that panics (in debug) if the parse engine tries
to peek beyond the declared limit, catching grammar correctness violations early.

### Benchmark impact

The `vs_serde_json` gap will narrow further after B4+B5. The streaming pipeline
eliminates the `Vec<Token>` peak-allocation spike visible in memory profiles and
reduces startup latency for large files (first token emitted to grammar before last
token is scanned).

### Dependencies

**Must be implemented after B4** — `Token<'src>` with `Cow<'src, str>` introduces a
lifetime that threads through `TokenSource` and the iterator adapter. Implementing
streaming before B4 would require doing the lifetime migration twice.
