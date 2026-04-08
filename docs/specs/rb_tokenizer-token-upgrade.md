# Spec: rb_tokenizer — Token Type Upgrade

**Status**: Ready for implementation
**Crate**: `rb_tokenizer`
**Module**: `rb_tokenizer::tokens`
**Depends on**: `rb_common::spans` (spec: `docs/specs/source-spans-and-labels.md`)
**Requirement source**: `docs/requirements/tokenizer-parser-integration-guidelines.md`
**Blocking**: diagnostics integration spec, pipeline contract spec

---

## Problem

The current `Token` struct records `line` and `column` but:

- has no `byte_offset` — the parser and renderer specs require byte offsets as first-class
- has no `SourceId` — needed to disambiguate tokens across multiple source buffers
- uses 1-based line/column stored directly — inconsistent with `SourcePosition` (which is 0-based in storage, converted to 1-based in display)
- `token_type` and `token_sub_type` are `&'static str` — keep these; they are the tokenizer's lightweight kind vocabulary

The `Scanner` trait returns a `Token` with `consumed_len` on `ScanMatch`. After this upgrade:
- `Token` gains a `span: SourceSpan` field
- Scanners continue to return a minimal `ScanMatch`; the tokenizer loop is the only place that constructs final span-bearing tokens
- Line/column tracking in `Tokenizer::advance_cursor` is kept, but output flows into `SourcePosition` fields

---

## Decisions Made

| Question | Decision |
|---|---|
| 1-based vs 0-based storage | `SourcePosition` is 0-based in storage. Tokenizer loop converts from its 1-based tracking counters by subtracting 1 before constructing `SourcePosition`. |
| Remove `line`/`column` from Token | Yes. They are replaced by `span.start.line` and `span.start.column`. |
| `SourceId` assignment | The `Tokenizer` gains an optional `source_id: SourceId` field, defaulting to `SourceId::UNKNOWN`. Callers that care about multi-source pipelines set it via `Tokenizer::with_source_id`. |
| Scanner trait signature | Unchanged. Scanners return `Option<ScanMatch>` with no span; the tokenizer loop adds the span. |
| `ScanMatch::consumed_len` | Unchanged. Used to advance the cursor. |

---

## Current Token (before)

```rust
pub struct Token {
    pub token_type: &'static str,
    pub token_sub_type: Option<&'static str>,
    pub value: String,
    pub line: usize,    // 1-based
    pub column: usize,  // 1-based
}
```

---

## New Token (after)

```rust
// In rb_tokenizer::tokens::token

use rb_common::spans::{SourceSpan, SourceId, SourcePosition};

/// A recognized lexical unit produced by the tokenizer.
///
/// `token_type` and `token_sub_type` are the lightweight kind vocabulary;
/// they are `&'static str` for zero-copy identity comparison.
///
/// `span` carries the full source location for diagnostics and parser integration.
#[derive(Debug, PartialEq, Clone)]
pub struct Token {
    pub token_type: &'static str,
    pub token_sub_type: Option<&'static str>,
    pub value: String,
    /// The byte-exact source location of this token.
    /// `span.start.byte_offset` is the byte index of the first character.
    /// `span.end.byte_offset` is the exclusive byte index after the last character.
    pub span: SourceSpan,
}

impl Token {
    /// Convenience accessor: 1-based display line.
    pub fn display_line(&self) -> usize {
        self.span.start.display_line()
    }

    /// Convenience accessor: 1-based display column.
    pub fn display_column(&self) -> usize {
        self.span.start.display_column()
    }
}
```

---

## Tokenizer Loop Changes

The `Tokenizer` gains a `source_id` field and a builder method:

```rust
// In rb_tokenizer::tokenizers::tokenizer

pub struct Tokenizer {
    scanners: Vec<ScannerType>,
    config: TokenizerConfig,
    last_errors: RefCell<Option<Vec<TokenizationError>>>,
    /// Source identity for tokens produced by this tokenizer instance.
    /// Defaults to `SourceId::UNKNOWN` for callers that do not manage
    /// multi-source pipelines.
    source_id: SourceId,
}

impl Tokenizer {
    pub fn new() -> Self {
        Tokenizer {
            scanners: Vec::new(),
            config: TokenizerConfig::default(),
            last_errors: RefCell::new(None),
            source_id: SourceId::UNKNOWN,
        }
    }

    /// Sets the source identity used on all tokens produced by this instance.
    pub fn with_source_id(mut self, id: SourceId) -> Self {
        self.source_id = id;
        self
    }
}
```

Inside `tokenize()`, replace the `Token { line, column, ... }` construction with:

```rust
// Before calling advance_cursor, capture the start position:
let start = SourcePosition {
    byte_offset: start,               // from chars.peek() byte index
    line: current_line - 1,           // convert 1-based to 0-based
    column: current_column - 1,       // convert 1-based to 0-based
};

// After advance_cursor:
let end = SourcePosition {
    byte_offset: start.byte_offset + token_len,
    line: current_line - 1,
    column: current_column - 1,
};

let span = SourceSpan {
    source_id: self.source_id,
    start,
    end,
};

let token_with_span = Token {
    token_type: token.token_type,
    token_sub_type: token.token_sub_type,
    value: token.value,
    span,
};
```

---

## Backward Compatibility

All existing tests access `token.line` and `token.column`. After this change:

- Replace `token.line` with `token.display_line()`
- Replace `token.column` with `token.display_column()`

These accessors preserve the 1-based semantics that tests already assert on.

---

## Cargo.toml Change

```toml
[dependencies]
rb_common = { path = "../rb_common" }
```

---

## Files Changed

| File | Change |
|---|---|
| `crates/rb_tokenizer/Cargo.toml` | Add `rb_common` dependency |
| `crates/rb_tokenizer/src/tokens/token.rs` | Replace `line`/`column` with `span: SourceSpan` |
| `crates/rb_tokenizer/src/tokenizers/tokenizer.rs` | Add `source_id` field; update token construction in `tokenize()` loop |
| All test files using `token.line` / `token.column` | Replace with `token.display_line()` / `token.display_column()` |
