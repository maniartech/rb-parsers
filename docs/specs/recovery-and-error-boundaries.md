# Spec: Recovery and Error Boundaries

**Status**: Ready for implementation
**Module**: `rb_common::recovery`
**Depends on**: `rb_common::spans`
**Requirement source**: `docs/requirements/recovery-and-error-boundaries.md`

---

## Decisions Made

| Question | Decision |
|---|---|
| Grammar-authored vs framework boundaries | Both. Framework provides a built-in `RecoveryBoundarySet::common()` with sensible defaults; grammar authors may extend or replace it |
| Recovered node representation | A `RecoveredMarker` newtype wraps any parse result; the API uses `RecoveryState` to signal confidence level to callers |
| Unified vs separate error budgets | Separate budgets (tokenizer, parser) shared under one `DiagnosticsContext`; each layer owns its budget counter but reports through the shared context |

---

## Module Layout

```
rb_common::recovery
├── RecoveryMode
├── RecoveryConfig
├── RecoveryBoundaryKind
├── RecoveryBoundary
├── RecoveryBoundarySet
├── RecoveryAction
├── RecoveryState
└── RecoveredMarker<T>
```

---

## Types

### RecoveryMode

High-level strategy selection.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RecoveryMode {
    /// Abort on the first error and return immediately.
    FailFast,
    /// Continue parsing up to configured limits when well-defined recovery
    /// boundaries exist.
    #[default]
    ContinueBounded,
}
```

---

### RecoveryConfig

Per-pipeline recovery tuning. The defaults are chosen for interactive tooling
and editor workflows.

```rust
#[derive(Debug, Clone)]
pub struct RecoveryConfig {
    pub mode: RecoveryMode,
    /// Maximum total error count before the pipeline stops attempting recovery.
    /// `0` means no limit (use with caution).
    pub max_errors: usize,
    /// Maximum number of tokens the parser may skip in a single recovery step.
    pub max_recovery_skips: usize,
}

impl Default for RecoveryConfig {
    fn default() -> Self {
        RecoveryConfig {
            mode: RecoveryMode::ContinueBounded,
            max_errors: 20,
            max_recovery_skips: 50,
        }
    }
}

impl RecoveryConfig {
    /// Returns a config suitable for strict batch validation.
    pub fn fail_fast() -> Self {
        RecoveryConfig {
            mode: RecoveryMode::FailFast,
            max_errors: 1,
            max_recovery_skips: 0,
        }
    }
}
```

---

### RecoveryBoundaryKind

Structural category of a synchronization point.

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryBoundaryKind {
    /// Comma or similar item-separator token.
    Separator,
    /// Closing delimiter: `]`, `}`, `)`.
    ClosingDelimiter,
    /// Sentence or statement terminator: `;`, newline (when significant).
    StatementTerminator,
    /// Top-level block boundary: end of a named definition or top-level form.
    BlockBoundary,
    /// Language-defined custom boundary. The string is a short noun phrase
    /// suitable for inclusion in a diagnostic message.
    Custom(&'static str),
}
```

---

### RecoveryBoundary

A single located boundary where the parser may safely resume.

```rust
use crate::spans::SourceSpan;

#[derive(Debug, Clone)]
pub struct RecoveryBoundary {
    pub kind: RecoveryBoundaryKind,
    /// The span of the token or syntactic element that constitutes this
    /// boundary, if known. May be `None` for grammar-declared boundaries that
    /// have not yet been encountered in source.
    pub span: Option<SourceSpan>,
}

impl RecoveryBoundary {
    pub fn new(kind: RecoveryBoundaryKind) -> Self {
        RecoveryBoundary { kind, span: None }
    }

    pub fn with_span(kind: RecoveryBoundaryKind, span: SourceSpan) -> Self {
        RecoveryBoundary { kind, span: Some(span) }
    }
}
```

---

### RecoveryBoundarySet

A named, composable set of boundary kinds. Grammar authors can start from
`common()` and extend.

```rust
#[derive(Debug, Clone, Default)]
pub struct RecoveryBoundarySet {
    pub kinds: Vec<RecoveryBoundaryKind>,
}

impl RecoveryBoundarySet {
    /// A sensible default for most collection-oriented languages: separators,
    /// closing delimiters, and statement terminators.
    pub fn common() -> Self {
        RecoveryBoundarySet {
            kinds: vec![
                RecoveryBoundaryKind::Separator,
                RecoveryBoundaryKind::ClosingDelimiter,
                RecoveryBoundaryKind::StatementTerminator,
            ],
        }
    }

    /// An aggressive set that includes block boundaries for top-level recovery.
    pub fn with_block_boundaries() -> Self {
        let mut s = Self::common();
        s.kinds.push(RecoveryBoundaryKind::BlockBoundary);
        s
    }

    pub fn add(mut self, kind: RecoveryBoundaryKind) -> Self {
        self.kinds.push(kind);
        self
    }

    pub fn is_empty(&self) -> bool {
        self.kinds.is_empty()
    }
}
```

---

### RecoveryAction

Documents what the parser did during a recovery step, so it can be attached
to diagnostics and parse results.

```rust
use crate::spans::SourceSpan;

#[derive(Debug, Clone)]
pub struct RecoveryAction {
    /// Human-readable description: "skipped 3 tokens to next `}`"
    pub description: String,
    /// The span of source that was skipped or mutated.
    pub skipped_span: Option<SourceSpan>,
    /// The recovery boundary the parser resumed at.
    pub resumed_at: Option<RecoveryBoundary>,
    /// Whether a synthetic token was inserted rather than real input consumed.
    pub was_synthetic_insertion: bool,
    /// Whether the parse results following this recovery should be considered
    /// fully trusted.
    pub confidence_degraded: bool,
}
```

---

### RecoveryState

The outcome of a recovery attempt. Parsers return this alongside their partial
result so callers can inspect confidence without ad-hoc boolean return codes.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryState {
    /// Parse succeeded without any recovery step.
    Clean,
    /// Parse succeeded after one or more bounded recovery steps. Results are
    /// likely still useful but should be treated with reduced trust.
    Recovered,
    /// Recovery was attempted but confidence in subsequent results is too low
    /// to continue safely. Callers should treat results as partial.
    PartialOnly,
    /// Recovery limit was hit. No further results are available.
    Exhausted,
}

impl RecoveryState {
    /// Returns true when results are safe to use for production purposes.
    pub fn is_usable(self) -> bool {
        matches!(self, RecoveryState::Clean | RecoveryState::Recovered)
    }
}
```

---

### RecoveredMarker\<T\>

Wraps any parse result together with its recovery state and the actions that
produced it. This is the canonical "partial result" type.

```rust
#[derive(Debug, Clone)]
pub struct RecoveredMarker<T> {
    pub value: T,
    pub state: RecoveryState,
    pub actions: Vec<RecoveryAction>,
}

impl<T> RecoveredMarker<T> {
    pub fn clean(value: T) -> Self {
        RecoveredMarker { value, state: RecoveryState::Clean, actions: Vec::new() }
    }

    pub fn recovered(value: T, actions: Vec<RecoveryAction>) -> Self {
        RecoveredMarker { value, state: RecoveryState::Recovered, actions }
    }

    pub fn partial(value: T, actions: Vec<RecoveryAction>) -> Self {
        RecoveredMarker { value, state: RecoveryState::PartialOnly, actions }
    }

    pub fn is_clean(&self) -> bool {
        self.state == RecoveryState::Clean
    }

    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> RecoveredMarker<U> {
        RecoveredMarker { value: f(self.value), state: self.state, actions: self.actions }
    }
}
```

---

## Error Budget Separation

Each pipeline layer maintains an independent counter:

```rust
#[derive(Debug, Clone, Default)]
pub struct ErrorBudget {
    pub error_count: usize,
    pub max_errors: usize,
    pub skip_count: usize,
    pub max_skips: usize,
}

impl ErrorBudget {
    pub fn from_config(config: &RecoveryConfig) -> Self {
        ErrorBudget {
            error_count: 0,
            max_errors: config.max_errors,
            skip_count: 0,
            max_skips: config.max_recovery_skips,
        }
    }

    /// Returns true when the budget allows another recovery attempt.
    pub fn can_recover(&self) -> bool {
        (self.max_errors == 0 || self.error_count < self.max_errors)
            && (self.max_skips == 0 || self.skip_count < self.max_skips)
    }

    pub fn record_error(&mut self) {
        self.error_count += 1;
    }

    pub fn record_skip(&mut self, count: usize) {
        self.skip_count += count;
    }
}
```

Tokenizer and parser each own an `ErrorBudget`. Both report diagnostics through
a shared `DiagnosticsContext` (defined in the diagnostics-runtime spec) so the
final diagnostic stream is ordered and coherent.

---

## Implementation Notes

- `RecoveredMarker<T>` is the standard return type for any public parser function
  that can produce partial results. For the simple fast-path where recovery never
  triggers, `RecoveredMarker::clean(value)` adds no overhead beyond a state
  discriminant.
- `RecoveryAction` values should be attached to the nearest enclosing diagnostic
  as secondary labels or as `Suggestion::Manual` entries, not emitted as
  standalone diagnostics.
- Grammar authors declare their boundary sets when defining grammar rules.
  The parser core queries the active `RecoveryBoundarySet` during a committed
  failure to locate the next safe resume point.
- The framework default for interactive use is `RecoveryConfig::default()` which
  is `ContinueBounded` with a 20-error limit. Production batch validation should
  use `RecoveryConfig::fail_fast()` or a custom limit.
# Recovery and Error Boundaries

## Objective

Define how tokenizer and parser can continue after errors in a disciplined way so the framework can collect more useful diagnostics without destroying trust in later results.

This spec is about recovery strategy, resume boundaries, and error collection behavior rather than one specific parser syntax.

## Core Principle

Continue-on-error is valuable only when it is bounded.

The framework should not force a false choice between:

- stop on first error
- continue blindly no matter what happened

Industry best practice is the middle path:

- continue when there is a well-defined recovery boundary
- stop or downgrade confidence when recovery becomes too speculative

## Why This Matters

For many real inputs, especially collections and repeated structures, users need to know more than the first failure.

Example:

- a list contains ten objects
- object three is malformed
- objects four through ten may still be valid

The framework should be able to report the failure in object three and still validate later objects if the parser can resume at a trustworthy boundary.

## Recommended Modes

The pipeline should support at least these conceptual modes:

```rust
pub enum RecoveryMode {
    FailFast,
    ContinueBounded,
}
```

Possible higher-level parser configuration:

```rust
pub struct RecoveryConfig {
    pub mode: RecoveryMode,
    pub max_errors: usize,
    pub max_recovery_skips: usize,
}
```

The exact API may differ, but the design should make the mode explicit.

## Tokenizer Recovery Best Practices

Tokenizer recovery should stay conservative.

Good tokenizer recovery cases:

- skip one unrecognized character and continue when configured to do so
- stop at a known lexical boundary after unterminated content
- emit an error and resume at a structurally meaningful delimiter when safe

Bad tokenizer recovery cases:

- guessing grammar intent
- consuming large arbitrary regions just to keep going
- rewriting token meaning speculatively

Tokenizer recovery should prioritize preserving downstream trust.

## Parser Recovery Best Practices

Parser can recover more aggressively than tokenizer because it understands structure.

Common industry strategies:

1. panic-mode recovery to synchronization tokens
2. phrase-level recovery for well-understood local mistakes
3. delimiter-aware recovery for nested structures
4. bounded skip counts to prevent runaway cascading errors

Good synchronization points include:

- commas between collection items
- closing delimiters such as `]`, `}`, `)`
- semicolons or statement terminators
- block boundaries

## Error Boundaries

An error boundary is a place where the framework can resume with acceptable confidence.

Likely shape:

```rust
pub struct RecoveryBoundary {
    pub kind: RecoveryBoundaryKind,
    pub span: Option<SourceSpan>,
}

pub enum RecoveryBoundaryKind {
    Separator,
    ClosingDelimiter,
    StatementTerminator,
    BlockBoundary,
    Custom(&'static str),
}
```

The parser should treat these as structural landmarks, not just arbitrary next tokens.

## Resume Rules

Recovery should document:

1. where parsing resumes
2. what tokens or input were skipped
3. whether any synthetic insertion or deletion was assumed
4. whether subsequent parse results are still considered high confidence

This makes recovery behavior understandable to users and maintainers.

## Diagnostics and Recovery

Recovery actions should be reflected in diagnostics when useful.

Examples:

- note that parsing resumed at the next object separator
- secondary label showing the enclosing object or collection
- hint explaining that the parser skipped to the next safe boundary

The framework should avoid emitting a long chain of low-value follow-on diagnostics after one broken region.

## Partial Results

When recovery succeeds, the framework may still produce useful partial results.

Best practice:

- preserve successful siblings or later collection items when confidence remains acceptable
- mark recovered nodes or subtrees distinctly if the API exposes parse results downstream
- make it possible for tooling to differentiate fully valid regions from recovered regions

## Recommended Defaults

Suggested default stance:

- interactive tooling and editor workflows: `ContinueBounded`
- strict batch validation and some benchmarking workflows: configurable `FailFast`
- parser should prefer one or a few high-confidence recovery steps over unlimited continuation

The framework should not default to blind best-effort continuation without limits.

## Testing Guidance

Recovery should be tested with:

- isolated local syntax errors
- repeated collection items where one item fails and later items remain valid
- nested delimiter failures
- malformed input that should force termination rather than noisy continuation
- maximum-error and maximum-skip thresholds

Tests should assert both diagnostics and what parsing resumed on.

## Industry Best Practices Summary

The best practice is:

1. precise primary error
2. bounded recovery
3. explicit synchronization boundary
4. continued parsing only while confidence remains acceptable
5. suppression of obvious cascades
6. preservation of later valid regions when possible

What is not best practice:

1. stop on first error in all modes
2. continue on error without boundary support
3. produce many low-trust follow-on diagnostics
4. hide recovery actions from users and maintainers

## Open Questions

1. Should recovery boundaries be grammar-authored only, or should the framework provide common default boundary sets?
2. How should recovered nodes be represented in public parse results?
3. Should tokenizer and parser share one unified error budget, or keep separate budgets under a shared diagnostics context?