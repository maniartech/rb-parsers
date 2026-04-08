# Spec: rb_tokenizer — Diagnostics Integration

**Status**: Ready for implementation
**Crate**: `rb_tokenizer`
**Modules**: `rb_tokenizer::tokens`, `rb_tokenizer::tokenizers`
**Depends on**:
- `rb_common::diagnostics` (spec: `docs/specs/diagnostics-runtime.md`)
- `rb_common::catalog` (spec: `docs/specs/error-catalog-and-compatibility.md`)
- `rb_tokenizer` token upgrade (spec: `docs/specs/rb_tokenizer-token-upgrade.md`)
**Requirement source**: `docs/requirements/error-system.md`, `docs/requirements/tokenizer-parser-integration-guidelines.md`

---

## Problem

The current tokenizer:

- returns `Result<Vec<Token>, Vec<TokenizationError>>` — errors are stringly-typed and carry no spans
- uses `continue_on_error: bool` and `error_tolerance_limit: usize` in `TokenizerConfig` — this duplicates the `RecoveryConfig` model from `rb_common::recovery`
- prints directly via `eprintln!` in `RegexScanner::normalize_pattern` — violates the "no direct print" requirement
- stores errors in `RefCell<Option<Vec<TokenizationError>>>` — makes sharing with a parser hard
- error format strings bake line/column into the message text rather than using structured spans

---

## Decisions Made

| Question | Decision |
|---|---|
| `tokenize()` signature | Change to accept `&mut DiagnosticsContext`. Return `Vec<Token>` always (empty on total failure). Callers inspect `ctx.has_errors()`. |
| `continue_on_error` / `error_tolerance_limit` | Keep in `TokenizerConfig` for now as a simple convenience layer. Internally they map onto `RecoveryConfig` semantics but the config struct change is backward-compatible. |
| `last_errors` field | Remove. Callers use the passed `DiagnosticsContext` instead. |
| `eprintln!` in `RegexScanner` | Remove. Emit a `Diagnostic` via a provided `DiagnosticsContext` or return `Err(TokenizationError::InvalidRegexPattern)` and let the tokenizer loop emit the diagnostic. The `add_regex_scanner` builder path continues to return `Err`. |
| `TokenizationError` enum | Keep as an internal type used by scanner code and `add_regex_scanner`. Do not expose it in the public tokenize return type. |
| Catalog dependency | `rb_tokenizer` depends on the `RBT` catalog (defined in `rb_tokenizer-error-catalog.md`). |

---

## New `tokenize()` Signature

```rust
/// Tokenizes `input`, emitting any diagnostics into `ctx`.
///
/// Always returns a `Vec<Token>`. On total failure, the returned list may
/// be empty. Callers should check `ctx.has_errors()` after return.
pub fn tokenize(&self, input: &str, ctx: &mut DiagnosticsContext) -> Vec<Token>
```

The old `Result<Vec<Token>, Vec<TokenizationError>>` signature is removed from the public API. A migration shim may be offered temporarily:

```rust
/// Compatibility shim — returns Err when the context collected any errors.
/// Prefer `tokenize()` with a `DiagnosticsContext` for new code.
#[deprecated(note = "Use tokenize() with DiagnosticsContext instead")]
pub fn tokenize_compat(&self, input: &str) -> Result<Vec<Token>, Vec<Diagnostic>> {
    let mut ctx = DiagnosticsContext::collecting();
    let tokens = self.tokenize(input, &mut ctx);
    if ctx.has_errors() {
        Err(ctx.take_all())
    } else {
        Ok(tokens)
    }
}
```

---

## Unrecognized Token Diagnostic

Replace:

```rust
let error = TokenizationError::UnrecognizedToken(
    format!("Unrecognized token at line {}, column {}: '{}'", ...)
);
errors.push(error);
```

With:

```rust
use rb_common::diagnostics::DiagnosticBuilder;
use rb_common::spans::{DiagnosticLocation, SourceSpan, SourcePosition};
use crate::catalog::{RBT_CATALOG, codes};

let location = DiagnosticLocation::Real(SourceSpan {
    source_id: self.source_id,
    start: SourcePosition {
        byte_offset: start,
        line: current_line - 1,
        column: current_column - 1,
    },
    end: SourcePosition {
        byte_offset: start + next_char.len_utf8(),
        line: current_line - 1,
        column: current_column,
    },
});

let diagnostic = DiagnosticBuilder::from_template(
    &RBT_CATALOG,
    codes::RBT_UNRECOGNIZED_CHAR,
    format!("unrecognized character `{next_char}` at this position"),
)
.primary(location)
.build();

ctx.emit(diagnostic);
```

---

## Invalid Regex Pattern at Registration

`add_regex_scanner` returns `Err(TokenizationError::InvalidRegexPattern { ... })` at scanner
registration time, before any input is seen. This path stays as-is because it happens outside
the tokenize loop. The caller is expected to handle the `Err` at setup time.

However, the `eprintln!` in `RegexScanner::normalize_pattern` is replaced with a
`DiagnosticsContext`-based warning — but only when a context is available. The auto-prefix
behavior is kept; the silent side effect (`eprintln!`) is removed.

Since `RegexScanner::new` does not accept a diagnostics context, the normalization warning is
surfaced as a field on the scanner and reported by `Tokenizer::add_regex_scanner` if the
caller passes an optional context:

```rust
pub fn add_regex_scanner_with_ctx(
    &mut self,
    pattern: &str,
    token_type: &'static str,
    sub_token_type: Option<&'static str>,
    ctx: &mut DiagnosticsContext,
) -> Result<&mut Self, TokenizationError>
```

The original `add_regex_scanner` continues to work without a context; the normalization
warning in that path is suppressed rather than printed.

---

## Remove `last_errors`

```rust
// Remove from Tokenizer struct:
last_errors: RefCell<Option<Vec<TokenizationError>>>,

// Remove from tokenize() body:
*self.last_errors.borrow_mut() = None;
*self.last_errors.borrow_mut() = Some(errors.clone());

// Remove public method:
pub fn last_errors(&self) -> Option<Vec<TokenizationError>> { ... }
```

Callers that previously used `tokenizer.last_errors()` should instead read
`ctx.collected()` or `ctx.errors()` after calling `tokenize()`.

---

## TokenizerConfig — Backward Compatibility

`TokenizerConfig` keeps `continue_on_error` and `error_tolerance_limit` as-is.
Inside `tokenize()` they gate `ctx.has_errors()` and the per-step skip logic:

```rust
// In tokenize() loop, unrecognized character path:
ctx.emit(diagnostic);

if self.config.continue_on_error
    && ctx.error_count() < self.config.error_tolerance_limit
{
    chars.next();
    current_column += 1;
} else {
    break;
}
```

This preserves the existing behavior while using the new diagnostics context internally.

---

## Files Changed

| File | Change |
|---|---|
| `crates/rb_tokenizer/Cargo.toml` | Add `rb_common` dependency (if not already from token upgrade) |
| `crates/rb_tokenizer/src/tokenizers/tokenizer.rs` | Remove `last_errors`; update `tokenize()` signature; emit diagnostics via `DiagnosticsContext`; add `tokenize_compat()` shim |
| `crates/rb_tokenizer/src/scanners/regex_scanner.rs` | Remove `eprintln!`; note normalization silently |
| All test files using `tokenize().unwrap()` | Update to `tokenize()` + `DiagnosticsContext::collecting()` |
| All test files inspecting `last_errors()` | Replace with `ctx.collected()` |
