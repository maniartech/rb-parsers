# P1 — Performance Issues

**Priority**: P1 — These issues directly cause the 7–12× throughput gap versus
`serde_json`. Address in the order listed: **B4** and **B5** account for the majority
of the gap and should be fixed first.

**Back to**: [Audit Index](README.md)

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
