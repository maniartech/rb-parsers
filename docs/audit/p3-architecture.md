# P3 — Architecture Issues

**Priority**: P3 — These issues will create larger refactoring debt later if ignored.
They do not cause immediate incorrect output (unlike P0) but make the codebase harder
to maintain, extend, and integrate.

**Back to**: [Audit Index](README.md)

---

## A4 · Two incompatible `RecoveryConfig` structs — the common one is dead

**Layer**: `rb_common`, `rb_parser`
**Files**: `crates/rb_common/src/recovery.rs`, `crates/rb_parser/src/profiles.rs`
**Symptom**: There are two structs both named `RecoveryConfig` with different fields.
The `rb_common` version is never used by `rb_parser`.

### Root Cause

```rust
// rb_common/src/recovery.rs
pub struct RecoveryConfig {
    pub mode:               RecoveryMode,
    pub max_errors:         Option<usize>,
    pub max_recovery_skips: Option<usize>,
}

// rb_parser/src/profiles.rs
pub struct RecoveryConfig {
    pub max_recovery_steps: usize,
    pub warn_on_limit:      bool,
}
```

The `rb_common` struct also defines `RecoveryMode`, `RecoveryAction`, `RecoveryHint`,
`RecoveryContext`, and `RecoveryOutcome` — a rich recovery framework. None of these
are used by the parser. The parser declares its own minimal variant independently.

### Impact

- A caller who sets `rb_common::recovery::RecoveryConfig` on a context they build will
  have their configuration silently ignored by the parser.
- Adding a feature to the common recovery model does not affect the parser.
- There are two separate places to update when recovery behaviour needs changing.

### Fix

Consolidate into one `RecoveryConfig` in `rb_common`. Migrate `rb_parser` to import
and use it. The parser-specific fields (`warn_on_limit`) can be added to the common
struct as optional extensions, or defined in `rb_parser` as a separate
`ParseRecoveryOptions` that wraps the common config:

```rust
// rb_parser/src/profiles.rs
pub struct ParseRecoveryOptions {
    pub config:       rb_common::recovery::RecoveryConfig,
    pub warn_on_limit: bool,
}
```

---

## A6 · `RendererSuitability` / renderer selection registry not implemented

**Layer**: `rb_common`
**File**: `crates/rb_common/src/render.rs`
**Symptom**: `DiagnosticRenderer::suitability()` exists as a trait method but there is
no registry that uses it.

### Root Cause

```rust
// render.rs
pub trait DiagnosticRenderer {
    fn suitability(&self, env: &EnvironmentSnapshot) -> RendererSuitability;
    fn render(&self, diag: &Diagnostic, options: &RenderOptions, source: &str) -> String;
}
```

`RendererSuitability` (`Optimal`, `Degraded`, `Incompatible`) is defined. Multiple
renderer implementations exist (`PlainRenderer`, `AnnotatedRenderer`, etc.). But there
is no `RendererRegistry` or `select_best_renderer(env: &EnvironmentSnapshot)` function
that iterates registered renderers, calls `suitability()`, and selects the best one.

### Impact

The entire `suitability()` infrastructure is structurally dead. Callers must always
pick a renderer explicitly. The intended "auto-select best renderer for the current
terminal/CI environment" workflow is not possible.

### Fix

Implement a `RendererRegistry`:

```rust
pub struct RendererRegistry {
    renderers: Vec<Box<dyn DiagnosticRenderer>>,
}

impl RendererRegistry {
    pub fn default() -> Self {
        Self {
            renderers: vec![
                Box::new(AnnotatedRenderer::new()),
                Box::new(PlainRenderer::new()),
            ]
        }
    }

    pub fn select(&self, env: &EnvironmentSnapshot) -> &dyn DiagnosticRenderer {
        self.renderers
            .iter()
            .filter_map(|r| {
                let s = r.suitability(env);
                if s != RendererSuitability::Incompatible { Some((s, r.as_ref())) } else { None }
            })
            .max_by_key(|(s, _)| *s)
            .map(|(_, r)| r)
            .unwrap_or_else(|| &*self.renderers.last().unwrap())
    }
}
```

---

## A7 · `RenderRequest` carries `EnvironmentSnapshot` but `render()` never receives it

**Layer**: `rb_common`
**File**: `crates/rb_common/src/render.rs`
**Symptom**: An `EnvironmentSnapshot` is stored in `RenderRequest` but is not passed
to `DiagnosticRenderer::render()`.

### Root Cause

```rust
pub struct RenderRequest {
    pub diagnostic:  Diagnostic,
    pub source:      String,
    pub options:     RenderOptions,
    pub env_snapshot: EnvironmentSnapshot,   // ← collected but discarded
}

// render.rs — DiagnosticRenderer trait
fn render(&self, diag: &Diagnostic, options: &RenderOptions, source: &str) -> String;
//                                                                 ↑ no env here
```

The `RenderRequest` is intended to be a self-contained render job (suitable for
batching, async processing, etc.). But the `render()` method signature does not accept
the env snapshot, so any renderer that wants to adapt output based on terminal width
or colour support cannot access it.

### Fix

Pass the env snapshot to `render()`:

```rust
fn render(
    &self,
    diag:    &Diagnostic,
    options: &RenderOptions,
    source:  &str,
    env:     &EnvironmentSnapshot,   // ← added
) -> String;
```

Or, refactor the `render()` method to accept the full `RenderRequest` as a single
parameter:

```rust
fn render(&self, request: &RenderRequest) -> String;
```

This second form is cleaner — it allows `RenderRequest` to evolve without changing
the trait signature.

---

## C3 · Dead `let _stack` allocation in `check_left_recursion`

**Layer**: `rb_parser`
**File**: `crates/rb_parser/src/grammar/combinator.rs`
**Symptom**: A `Vec<String>` is allocated and immediately dropped inside
`check_left_recursion` without being read.

### Root Cause

```rust
fn check_left_recursion<R>(rules: &RulesMap<R>, start_key: &str) {
    let _stack: Vec<String> = vec![start_key.clone()];   // ← allocated and dropped
    // ... (rest of function does not use _stack)
}
```

This was likely scaffolding for a stack-trace-based left-recursion cycle detector
that was never completed.

### Impact

Every call to `Grammar::compile()` allocates this Vec unnecessarily. More importantly,
left-recursion detection is incomplete — `check_left_recursion` either does nothing
or does not produce the cycle path it intended to produce. A left-recursive grammar
will silently recurse at parse time until the thread stack overflows.

### Fix

Either complete the left-recursion checker (track the actual recursion path and return
a useful error message showing the cycle), or remove the dead code entirely:

```rust
// Remove the dead allocation
// If left-recursion checking is not implemented, document that limitation explicitly
fn check_left_recursion<R>(_rules: &RulesMap<R>, _start_key: &str) {
    // TODO: implement cycle detection using a visited-set DFS
    // Until then, left-recursive grammars will panic at parse time with a stack overflow
}
```

---

## C7 · Redundant `unsafe impl Send + Sync` on `CompiledParser`

**Layer**: `rb_parser`
**File**: `crates/rb_parser/src/grammar/combinator.rs`
**Symptom**: `unsafe impl Send` and `unsafe impl Sync` are declared manually even though
the contained `Arc<dyn ParseFn: Send+Sync>` already satisfies both.

### Root Cause

```rust
// combinator.rs
unsafe impl Send for CompiledParser { }
unsafe impl Sync for CompiledParser { }
```

`CompiledParser` contains `Arc<...>` fields. `Arc<T: Send+Sync>` is already `Send+Sync`.
The manual `unsafe impl` blocks were likely added to suppress compiler errors during
an earlier development stage when the inner type was not properly bounded.

### Impact

The `unsafe impl` blocks assert thread-safety without the compiler verifying it. If a
future contributor adds a non-Send field (e.g. `Rc<...>`, raw pointer, `RefCell<...>`)
to `CompiledParser`, the manual impls will suppress the compile error and produce
undefined behaviour at runtime.

### Fix

Remove the manual `unsafe impl` blocks and ensure the inner types have proper `Send+Sync`
bounds. If the compiler then flags a field as non-Send, that is a genuine soundness issue
to be fixed rather than suppressed.

---

## C17 · No `ProfileCatalog` registry – versioning infrastructure builds but nothing drives it

**Layer**: `rb_parser`
**File**: `crates/rb_parser/src/profiles.rs`
**Requirement source**: `framework-objectives.md` §Language Profile Management

### Root Cause

```rust
// profiles.rs
pub struct ResolvedProfile {
    pub id:       ResolvedProfileId,
    pub language: String,
    pub version:  LanguageVersion,
    pub mode:     ProfileMode,
    pub features: Vec<FeatureFlag>,
    pub guards:   Vec<VersionGuard>,
}

impl ResolvedProfile {
    pub fn simple(language: &str) -> Self { ... }   // ← convenience, not registry
}
```

There is no `ProfileCatalog` that:
- Stores named profiles (`"json"`, `"typescript"`, `"python3.12"`)
- Resolves `"typescript"` + `["strict"]` features to a specific `ResolvedProfile`
- Manages compiled grammar caching keyed by `ResolvedProfileId`

The `simple()` constructor creates one-off profiles. Any project that needs multiple
language profiles must manage the mapping and caching itself.

### Fix

Implement `ProfileCatalog` as a registry:

```rust
pub struct ProfileCatalog {
    profiles: HashMap<String, Vec<ResolvedProfile>>,
    cache:    HashMap<ResolvedProfileId, Arc<CompiledParser>>,
}

impl ProfileCatalog {
    pub fn register(&mut self, profile: ResolvedProfile, parser: CompiledParser) { ... }
    pub fn resolve(&self, language: &str, features: &[FeatureFlag]) -> Option<Arc<CompiledParser>> { ... }
}
```

---

## C20 · No `SourceIdRegistry` — `SourceId` is a raw public `u32`

**Layer**: `rb_parser` / `rb_common`
**File**: `crates/rb_common/src/spans.rs`
**Requirement source**: `framework-objectives.md` §Multi-file diagnostics

### Root Cause

```rust
// spans.rs
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct SourceId(pub u32);   // ← raw public field, no registry
```

`SourceId` is intended to identify a file (or input buffer) in multi-file workspaces.
It is a raw `u32` with no allocation or uniqueness guarantee. Callers assign IDs
manually. In a workspace with two files, if both callers decide `SourceId(1)` is their
file, diagnostics will conflate spans from both files.

### Impact

Multi-file parsing, IDEs managing a workspace, and test suites processing multiple
input fixtures all require correctly distinct `SourceId` values. With a raw public
field, collision is silent and produces incorrect diagnostic file attribution.

### Fix

Provide a `SourceRegistry` that allocates monotonically increasing IDs:

```rust
pub struct SourceRegistry {
    next_id: AtomicU32,
    files:   DashMap<SourceId, SourceInfo>,
}

pub struct SourceInfo {
    pub path:    Option<PathBuf>,
    pub content: String,
}

impl SourceRegistry {
    pub fn register(&self, path: Option<PathBuf>, content: String) -> SourceId {
        let id = SourceId(self.next_id.fetch_add(1, Ordering::Relaxed));
        self.files.insert(id, SourceInfo { path, content });
        id
    }
}
```

Make `SourceId::new()` (or the public field) `pub(crate)` so external callers must
go through `SourceRegistry`.

---

## D2 · No error catalog in `rb_tokenizer` — errors are stringly typed

**Layer**: `rb_tokenizer`
**File**: `crates/rb_tokenizer/src/**`
**Comparison**: `rb_parser/src/catalog.rs` uses typed error codes.

### Root Cause

`rb_parser` defines:
```rust
// rb_parser/src/catalog.rs
pub struct ParseErrorCode(&'static str);
pub const UNEXPECTED_TOKEN: ParseErrorCode = ParseErrorCode("E_UNEXPECTED_TOKEN");
```

`rb_tokenizer` has no equivalent — errors are raw `String` messages:
```rust
TokenizationError { message: String, ... }
```

### Impact

- Callers cannot test for a specific error without string-matching.
- Internationalization of error messages is impossible if the message is the identity.
- The split between a typed error catalog in the parser and stringly-typed errors in
  the tokenizer is an API inconsistency that users of both layers will notice.

### Fix

Create `crates/rb_tokenizer/src/catalog.rs` mirroring the parser's design:

```rust
pub struct TokenizationErrorCode(&'static str);
pub const UNMATCHED_INPUT:    TokenizationErrorCode = TokenizationErrorCode("T_UNMATCHED_INPUT");
pub const UNEXPECTED_CHAR:    TokenizationErrorCode = TokenizationErrorCode("T_UNEXPECTED_CHAR");
pub const UNCLOSED_BLOCK:     TokenizationErrorCode = TokenizationErrorCode("T_UNCLOSED_BLOCK");
pub const SCANNER_PANIC:      TokenizationErrorCode = TokenizationErrorCode("T_SCANNER_PANIC");
```

And change `TokenizationError` to carry `code: TokenizationErrorCode` alongside a
human-readable message.

---

## D3 · `GrammarError` implements `std::error::Error` but `CompiledParser` error path does not

**Layer**: `rb_parser`
**File**: `crates/rb_parser/src/grammar/combinator.rs`
**Symptom**: `GrammarError` derives/implements `std::error::Error`, but
`Grammar::compile()` returns `Result<CompiledParser, Vec<GrammarError>>` — a `Vec`
which does not itself implement `std::error::Error`. Callers using `?` propagation
cannot propagate grammar compilation errors.

### Fix

Either:

1. Define a `GrammarErrors(Vec<GrammarError>)` newtype that implements
   `std::error::Error` and `Display`.
2. Return `Result<CompiledParser, GrammarError>` where `GrammarError::Multiple(Vec<GrammarError>)`
   is an aggregation variant.

---

## D4 · Public enums missing `#[non_exhaustive]` — adding variants is semver-breaking

**Layer**: cross-cutting
**Affected types**: `ParseEvent`, `GrammarError`, `RecoveryAction`, `DiagnosticLocation`,
`ScannerType`, `TokenizationError`
**Symptom**: Any external crate that `match`es on these enums will break when a new
variant is added.

### Fix

Add `#[non_exhaustive]` to all public enums that are likely to grow:

```rust
#[non_exhaustive]
pub enum ParseEvent {
    TokenConsumed { ... },
    NodeStart { ... },
    NodeEnd { ... },
    Error(Box<Diagnostic>),
    RecoveryAttempt { ... },
    // future variants won't break downstream match statements
}
```

This is a semver-additive change: existing clients that already use `_ => {}` wildcards
are unaffected; others will get a compile-time reminder to handle new variants.
