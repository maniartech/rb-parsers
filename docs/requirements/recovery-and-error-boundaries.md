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