# Spec: Tokenizer–Parser Pipeline Contract

**Status**: Ready for implementation
**Crates**: `rb_tokenizer`, `rb_parser`, `rb_common`
**Requirement source**: `docs/requirements/tokenizer-parser-integration-guidelines.md`
**Depends on**:
- `rb_common::spans` (spec: `docs/specs/source-spans-and-labels.md`)
- `rb_common::diagnostics` (spec: `docs/specs/diagnostics-runtime.md`)
- `rb_tokenizer` token upgrade (spec: `docs/specs/rb_tokenizer-token-upgrade.md`)
- `rb_tokenizer` diagnostics integration (spec: `docs/specs/rb_tokenizer-diagnostics-integration.md`)

---

## Problem

`rb_parser` is currently empty. `rb_tokenizer` and `rb_parser` have no shared
interface. The requirement is to establish a clean, composable pipeline
contract that:

1. avoids duplicating source or span information between layers
2. passes one shared `DiagnosticsContext` so tokenizer and parser diagnostics
   form one coherent ordered output stream
3. defines what `rb_parser` receives as its input (a `TokenStream`)
4. makes the simple "tokenize then parse" path tiny while leaving the advanced
   multi-profile path available

---

## Decisions Made

| Question | Decision |
|---|---|
| Token stream type | A `TokenStream<'src>` struct wrapping `&'src [Token]` with the source text and `SourceId`. No eager AST. |
| Source text ownership | `TokenStream` borrows `&'src str`. The caller owns the source string. |
| Diagnostics ownership | One `DiagnosticsContext` created by the caller, passed by `&mut` to both tokenizer and parser. |
| Profile / mode resolution | Out of scope for Phase 1. `TokenStream` carries no profile. Profile gating is Phase 2. |
| Parser trait | `Parser` trait with one method: `parse(stream: &TokenStream<'_>, ctx: &mut DiagnosticsContext) -> ParseResult`. `ParseResult` is a newtype holding a `Cst` placeholder for Phase 2. |
| `rb_parser` Cargo deps | `rb_common` + `rb_tokenizer` (for `Token` and `SourceId` types). |

---

## Types

### `TokenStream<'src>` — in `rb_tokenizer::pipeline`

The canonical handoff from tokenizer to parser.

```rust
// New file: crates/rb_tokenizer/src/pipeline.rs

use crate::tokens::Token;
use rb_common::spans::SourceId;

/// The output of tokenization and the input to parsing.
///
/// `TokenStream` borrows the original source text so the parser can
/// extract slices or build spans without copying source bytes.
pub struct TokenStream<'src> {
    /// The original source text that was tokenized.
    pub source: &'src str,
    /// The source identity shared across all tokens in this stream.
    pub source_id: SourceId,
    /// The tokens produced by the tokenizer, in emission order.
    pub tokens: Vec<Token>,
}

impl<'src> TokenStream<'src> {
    pub fn new(source: &'src str, source_id: SourceId, tokens: Vec<Token>) -> Self {
        TokenStream { source, source_id, tokens }
    }

    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }

    pub fn len(&self) -> usize {
        self.tokens.len()
    }

    /// Returns a slice of all tokens.
    pub fn as_slice(&self) -> &[Token] {
        &self.tokens
    }

    /// Returns the token at index `i`, or `None` if out of range.
    pub fn get(&self, i: usize) -> Option<&Token> {
        self.tokens.get(i)
    }

    /// Returns the source text slice for a byte range.
    /// Panics in debug builds if the range is out of bounds; returns empty string
    /// in release builds.
    pub fn source_slice(&self, start: usize, end: usize) -> &str {
        debug_assert!(end <= self.source.len(), "source_slice: end out of bounds");
        debug_assert!(start <= end, "source_slice: start > end");
        if end <= self.source.len() && start <= end {
            &self.source[start..end]
        } else {
            ""
        }
    }
}
```

---

### `ParseResult` — in `rb_parser`

Phase 1 placeholder. Phase 2 replaces `Cst` with the real syntax tree type.

```rust
// In crates/rb_parser/src/lib.rs

use rb_common::diagnostics::DiagnosticsContext;

/// Phase 1 placeholder for a parsed syntax tree.
/// Will be replaced by a real `Cst` type in Phase 2.
pub struct ParseResult {
    /// True when parsing completed without any errors in the context.
    pub is_ok: bool,
    // Phase 2: pub cst: Cst,
}

impl ParseResult {
    pub fn success() -> Self {
        ParseResult { is_ok: true }
    }

    pub fn failure() -> Self {
        ParseResult { is_ok: false }
    }
}
```

---

### `Parser` trait — in `rb_parser`

```rust
// In crates/rb_parser/src/lib.rs

use rb_tokenizer::pipeline::TokenStream;
use rb_common::diagnostics::DiagnosticsContext;

pub trait Parser {
    /// Parses `stream` and emits any diagnostics into `ctx`.
    /// Returns a `ParseResult` regardless of success or failure.
    fn parse(&self, stream: &TokenStream<'_>, ctx: &mut DiagnosticsContext) -> ParseResult;
}
```

---

## Pipeline Entry Pattern

The canonical usage pattern for consuming code:

```rust
use rb_common::diagnostics::DiagnosticsContext;
use rb_common::spans::SourceId;
use rb_tokenizer::{Tokenizer, TokenizerConfig};
use rb_tokenizer::pipeline::TokenStream;

fn run_pipeline(source: &str) -> DiagnosticsContext {
    let mut ctx = DiagnosticsContext::collecting();

    // 1. Build the tokenizer
    let mut tokenizer = Tokenizer::new().with_source_id(SourceId(1));
    tokenizer.add_regex_scanner(r"^\d+", "Number", None).unwrap();
    // ... add more scanners

    // 2. Tokenize — diagnostics go into ctx
    let tokens = tokenizer.tokenize(source, &mut ctx);

    // 3. Build the token stream
    let stream = TokenStream::new(source, SourceId(1), tokens);

    // 4. Parse — same ctx, same source_id (Phase 2: parser.parse(&stream, &mut ctx))
    // Phase 1: parser not yet implemented

    ctx
}
```

---

## Cargo.toml Changes

### `rb_tokenizer/Cargo.toml`

```toml
[dependencies]
regex = "1.10.3"
rb_common = { path = "../rb_common" }
```

### `rb_parser/Cargo.toml`

```toml
[dependencies]
rb_common  = { path = "../rb_common" }
rb_tokenizer = { path = "../rb_tokenizer" }
```

### `rb_common/Cargo.toml`

No changes. `rb_common` has no crate dependencies.

---

## `rb_tokenizer/src/lib.rs` — Export

```rust
pub mod scanners;
pub mod tokens;
pub mod tokenizers;
pub mod utils;
pub mod catalog;
pub mod pipeline;   // NEW

pub use tokenizers::{Tokenizer, TokenizerConfig};
pub use pipeline::TokenStream;   // NEW
```

---

## `rb_parser/src/lib.rs` — Skeleton

Replace the stub with the real skeleton:

```rust
use rb_tokenizer::pipeline::TokenStream;
use rb_common::diagnostics::DiagnosticsContext;

pub struct ParseResult {
    pub is_ok: bool,
}

impl ParseResult {
    pub fn success() -> Self { ParseResult { is_ok: true } }
    pub fn failure() -> Self { ParseResult { is_ok: false } }
}

pub trait Parser {
    fn parse(&self, stream: &TokenStream<'_>, ctx: &mut DiagnosticsContext) -> ParseResult;
}
```

---

## Invariants Enforced by This Contract

1. **Source identity flows through**: `Tokenizer::source_id` → `Token::span.source_id` → `TokenStream::source_id`. All three must agree.
2. **Diagnostics are emission-ordered**: tokenizer emits first (lexical), parser emits second (syntactic). One `DiagnosticsContext` collects both in sequence.
3. **No re-tokenization**: parser receives `&TokenStream`; it does not call the tokenizer again.
4. **No direct printing**: neither `Tokenizer::tokenize` nor any `Parser::parse` implementation may call `eprintln!` or `println!` directly. All output goes through `DiagnosticsContext`.
5. **Parser is infallible in signature**: `parse()` returns `ParseResult`, never `Result<_, _>`. All errors are diagnostics, not panics or `Err` values.

---

## Files to Create / Modify

| File | Change |
|---|---|
| `crates/rb_tokenizer/src/pipeline.rs` | New file — `TokenStream<'src>` |
| `crates/rb_tokenizer/src/lib.rs` | Add `pub mod pipeline; pub use pipeline::TokenStream;` |
| `crates/rb_tokenizer/Cargo.toml` | Add `rb_common` dependency |
| `crates/rb_parser/src/lib.rs` | Replace stub with `ParseResult` + `Parser` trait |
| `crates/rb_parser/Cargo.toml` | Add `rb_common` + `rb_tokenizer` dependencies |
