# Tokenizer and Parser Integration Guidelines

## Objective

Define stable integration guidance for `rb_tokenizer` and `rb_parser` without locking the project into one parser syntax or one grammar authoring style.

This document is intentionally about best practices, architecture, and diagnostics behavior rather than surface syntax.

## Audience

This guidance is for:

- library maintainers designing the cross-crate boundary
- language authors building a tokenizer-parser pipeline
- contributors adding diagnostics, recovery, or new token kinds

## Core Principle

Tokenizer and parser should feel like one pipeline to the user, but they must keep a clean responsibility boundary.

That means:

- tokenizer owns lexical recognition and lexical diagnostics
- parser owns grammar, recovery, and syntax diagnostics
- shared diagnostics infrastructure must make both phases look coherent without blurring ownership

## Responsibilities

### Tokenizer Responsibilities

Tokenizer should own:

- token recognition
- token classification
- exact token spans
- lexical warnings and lexical errors
- delimiter matching rules that are fundamentally lexical

Tokenizer should not own:

- grammar validation
- precedence decisions
- statement or expression structure
- parser recovery policies

### Parser Responsibilities

Parser should own:

- grammar rules
- syntax errors
- recovery behavior
- multi-token contextual diagnostics
- suggestions that require syntactic understanding

Parser should not:

- re-tokenize raw source unless there is a compelling documented reason
- silently reinterpret lexical failures as syntax failures
- duplicate lexical diagnostics that tokenizer already emitted

## Integration Boundary

The preferred pipeline shape is:

1. create shared source and diagnostics context
2. resolve one parsing profile for the current language, version, and mode
3. tokenize input
4. pass tokens, source identity, resolved profile, and diagnostics context into parser
5. emit final diagnostics through one runtime path

This keeps output coherent while preserving subsystem ownership.

## Shared Profile Resolution

Tokenizer and parser must agree on the same resolved parsing profile.

Do:

- resolve the requested profile once at the pipeline entry point
- pass the same resolved profile into tokenizer and parser
- keep profile resolution deterministic and validated before deep parsing work begins
- validate profile compatibility and composition rules before tokenization or parser setup starts
- use profile-aware tokenization only when lexical syntax truly differs by version, mode, or feature set

Do not:

- let tokenizer and parser infer strictness or version independently
- hide profile selection behind scattered booleans across subsystems
- assume two profiles are compatible just because their names share a base version or mode label
- fork whole grammars when guarded rules or feature overlays would express the difference more clearly

The ecosystem should support combinations such as `v1 + strict`, `v1 + tolerant`, or `v2 + extension_x` as structured profiles rather than as unrelated bespoke parser instances.

## Shared Diagnostics Context

The best default is one diagnostics context shared across tokenization and parsing.

Do:

- create the diagnostics context at the pipeline entry point
- pass the same context into tokenizer and parser
- preserve emission order so lexical diagnostics appear before syntax diagnostics when that matches execution

Do not:

- create unrelated tokenizer and parser sinks by default
- print directly from scanners or parser rules
- merge diagnostics after the fact by string concatenation

## Source and Span Rules

Tokenizer is the source of truth for lexical extents. Parser should reuse that source information rather than reconstructing it loosely.

Do:

- attach spans to tokens as early as possible
- preserve source identity across all phases
- use secondary labels to connect parser failures to earlier lexical anchors
- preserve enough parser structure to identify the immediate owning region and useful ancestor scopes for nested diagnostics

Do not:

- drop token span information before parsing
- rebuild parser spans from guessed string offsets if token spans already exist
- treat line and column as sufficient without source identity and byte offsets
- flatten nested failures into one isolated token span when the enclosing structure is what gives the failure meaning

## Error Code Ownership

Error codes should stay aligned with subsystem responsibility.

Recommended ownership:

- tokenizer diagnostics use tokenizer-owned codes
- parser diagnostics use parser-owned codes
- framework and shared diagnostics behavior use common/framework codes

Do not place parser failures under tokenizer codes just because the parser happened to encounter them first.

## Hints, Notes, and Suggestions

Hints and suggestions should be used deliberately.

Use a hint when:

- the user can take a clear next step
- the failure mode is not obvious from the message alone

Use a note when:

- extra factual context helps explain the issue
- a related earlier location matters

Use a suggestion when:

- there is a concrete textual edit or replacement to propose
- tooling could show or apply the change meaningfully

Do not:

- attach generic hints to every diagnostic
- use framework fallback hints as filler text when the subsystem has insufficient context
- emit speculative machine-applicable suggestions for ambiguous parser recovery
- repeat the same message in the title, message, note, and hint fields

When user-authored hints are absent, subsystem integrations should prefer dedicated automatic hint providers that use tokenizer or parser context directly rather than relying on shallow framework-wide generic messages.

## Recovery Guidance

Recovery is primarily parser territory.

Best practices:

- emit one clear primary diagnostic for the root issue
- use recovery to continue analysis where it adds value
- avoid cascading noise by suppressing obvious follow-on diagnostics when possible
- use notes or secondary labels to explain inserted or skipped structure when helpful
- resume only at well-defined boundaries, not arbitrary token positions

Tokenizer recovery should remain conservative. If tokenization becomes too speculative, parser diagnostics will lose credibility.

## Continue-on-Error and Error Boundaries

Best practice is not binary fail-fast versus blind continuation.

The preferred model is:

1. fail fast when configured for strict or benchmark-oriented single-error modes
2. continue on error when configured for tooling and diagnostics-rich workflows
3. only continue when the subsystem has explicit recovery boundaries and resume rules

For nested collections such as arrays or object lists, parser recovery should usually resume at the next synchronization boundary, such as:

- item separators
- closing delimiters
- statement terminators
- block delimiters

This is how the framework can still report that one object failed while the next object parsed cleanly.

Do not continue blindly without boundary support. That produces noisy, misleading, low-trust diagnostics.

## Testing Guidance

Integration tests should validate the full pipeline, not just token counts or parse success.

Prefer asserting:

- token sequence and token spans
- parser result shape
- diagnostic codes
- primary spans and secondary labels where important
- presence of hints or suggestions when they are part of the expected UX

Avoid coupling too many tests to exact renderer wording unless wording stability is explicitly part of the contract.

## Diagnostics Framework Best Practices

Recommended top-level usage pattern:

```rust
let source_id = sources.add("input.rule", input_text);
let diagnostics = DiagnosticsContext::new(config);
let profile = profile_catalog.resolve(ParseProfileRequest::new("my_language"))?;

let tokens = tokenizer.tokenize_with_context(input_text, source_id, &profile, &diagnostics)?;
let result = parser.parse_with_context(&tokens, source_id, &profile, &diagnostics)?;

let emitted = diagnostics.collected();
```

The exact API may differ, but the integration principles should remain:

- caller creates shared context
- source identity is established once
- tokenizer and parser reuse the same context and the same resolved profile
- final rendering happens outside low-level logic

## Dos and Don'ts

### Do

- keep tokenizer and parser diagnostics in one coherent pipeline
- preserve token spans through parsing
- keep subsystem ownership of codes and messages clear
- resolve parsing profiles once and share them across the full pipeline
- use secondary labels to connect related positions
- use explicit synchronization and resume rules for continue-on-error workflows
- test real-language examples end to end

### Don't

- let tokenizer perform grammar work
- let parser guess lexical boundaries that tokenizer already knows
- emit directly to stderr from low-level code
- spread version and strict-mode handling across unrelated booleans and ad hoc conditionals
- continue after errors without clear recovery boundaries
- use suggestions for uncertain recovery behavior
- flood the user with duplicate follow-on diagnostics

## Open Questions

1. Should parser APIs require span-bearing tokens, or support a weaker fallback mode?
2. Should tokenizer emit trivia tokens, hidden trivia, or source maps for parser-side formatting and tooling use cases?
3. How should the pipeline expose partial results when diagnostics were emitted but parsing still succeeded well enough for tooling?