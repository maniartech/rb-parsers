# Framework Objectives

## Objective

Record the non-negotiable design goals for the `rb_parsers` framework so future API, runtime, diagnostics, and integration decisions can be judged against the same standard.

This document is intentionally about product and engineering goals, not parser syntax or grammar authoring style.

## Vision

When a user builds a parser with `rb_parsers`, the result should be:

- high performance on real workloads
- memory efficient under sustained parsing and tooling use cases
- competitive with or better than comparable parser frameworks on representative benchmarks
- strong in diagnostics, hints, and recovery guidance
- strong in framework-generated fallback hints when user-authored hints are absent
- pleasant to build with, test, debug, and extend
- approachable enough that a novice Rust programmer can build a serious parser without learning parser engine internals first
- reusable across host languages from one language definition when that materially reduces developer effort

The framework should aim to produce parsers that are not only fast, but also elegant to use and trustworthy in production tooling.

It should also make it easy to support real language variation such as versioned syntax, strict versus tolerant modes, and controlled feature combinations without forcing grammar authors into copy-pasted parser forks.

It should also support multiple serious consumption styles, including tree-oriented usage, visitor-style traversal, event streams, and future incremental tooling, without forcing every user into the same parser surface.

It should also make it possible to define one language or mini-DSL once and reuse or emit it across multiple host languages when that avoids repeated porting work.

The overall product direction should be: extremely easy defaults and a fast path for common parsers, with more advanced power available through rule-based configuration that remains learnable and implementable.

The easy path should also be the performance-safe path. Users should not need to opt into the right defaults after already learning the hard way.

## Primary Quality Goals

### 1. Performance

The framework should aim for excellent runtime performance in realistic parsing scenarios.

This means:

- low overhead abstractions
- efficient token and parse pipelines
- avoidance of accidental quadratic behavior
- attention to hot paths in tokenization, parsing, and diagnostics emission
- default parser semantics that favor deterministic, low-backtracking execution unless a grammar author explicitly asks for something more expensive
- a default CST representation compact enough that tree-first parsing does not become the accidental performance bottleneck
- AST or other higher-level structures should not be materialized eagerly unless the caller asks for them

Performance goals must be evaluated on representative grammars and inputs rather than synthetic toy examples alone.

### 2. Memory Efficiency

The framework should avoid unnecessary allocation, copying, and retention of intermediate state.

This means:

- source and token models should preserve spans efficiently
- diagnostics should carry rich information without creating avoidable bloat
- parser APIs should make ownership and borrowing strategies explicit where that matters
- syntax trees should avoid duplicated source text and avoid per-node allocation patterns that scale poorly
- trivia and source-preserving structure should be retained without forcing full object-heavy trees by default

### 3. Diagnostics Quality

The framework should provide elegant error handling, hinting, and recovery-oriented feedback.

This means:

- stable error codes
- high-quality labels and spans
- hierarchical context that shows the owning region and relevant ancestor scopes
- actionable hints and notes
- high-quality automatic hints that feel dedicated to the specific failure rather than generic filler
- bounded recovery that can continue collecting useful diagnostics without losing trust
- structured suggestions where appropriate
- coherent tokenizer-parser diagnostics through one shared runtime model

The user experience of a failure matters as much as the raw ability to detect it.

### 4. Developer Experience and Learnability

The framework should feel deliberate and pleasant to build on.

This means:

- APIs should be consistent and composable
- behavior should be testable and debuggable
- diagnostics should help framework users author grammars and integrations correctly
- novice Rust programmers should be able to author useful and eventually sophisticated parsers without first learning memoization, cut placement, or parser-recovery internals
- grammar authors should not need to reimplement the same DSL parser separately in every host language unless they explicitly choose backend-specific specialization
- common tasks should be straightforward without hiding important control from advanced users
- version, strictness, and feature-profile configuration should be simple for common cases and structured for advanced cases
- default parser usage should be simple, while advanced consumption models such as event or incremental workflows remain available when justified
- the simplest useful parser should require very little setup
- the best-documented path should also be the safest path for performance and diagnostics
- common grammar shapes such as delimited lists, grouped forms, and precedence-based expressions should have obvious combinators with good defaults
- portable grammar authoring and multi-target emission should reduce duplication rather than introducing a second, harder framework to learn
- advanced behavior should come from understandable rules and structured composition rather than opaque magic
- users should be able to grow from the easy path to the advanced path without needing to relearn the framework from scratch

### 5. Benchmark Credibility

Benchmark ambitions should be real and measurable.

The target is not vague claims of speed. The target is credible performance demonstrated through:

- published benchmark methodology
- representative competitor comparisons
- representative language and workload coverage
- stable regression tracking over time

If the framework claims to outperform others, that claim should be grounded in reproducible benchmark evidence.

## Secondary Goals

- predictable integration between tokenizer and parser layers
- clean subsystem boundaries
- strong testability for libraries and downstream language authors
- output modes suitable for CLI, logs, editors, and CI
- first-class support for multi-profile parsing across versions, modes, and dialect-like feature sets
- first-class support for multiple parser consumption models, including editor and streaming-oriented use cases
- a future path to backend-neutral grammar reuse and non-Rust parser backends when that reduces total developer effort
- a preference for Rust-backed portability surfaces such as C bindings, WebAssembly, and WASI when they preserve one implementation and reduce maintenance
- a layered learning curve where the default path is minimal and the advanced path is structured
- design clarity that allows long-term evolution without API chaos

## Non-Goals

The framework should not:

- optimize only for toy microbenchmarks while degrading real parser usability
- trade away diagnostics quality for small headline speed gains without clear justification
- lock itself into one parser surface syntax too early
- hide important failure or recovery behavior behind opaque defaults
- require users to understand cut placement, memoization strategy, or recovery internals before they can build a competent parser
- bake host-language-specific semantic actions into the canonical portable grammar layer by default
- make advanced features accessible only through ad hoc flags, boilerplate-heavy setup, or architecture forks

## Decision Rule

New design work should be evaluated against these questions:

1. Does it improve or preserve real performance?
2. Does it improve or preserve memory efficiency?
3. Does it improve or preserve diagnostics quality?
4. Does it improve or preserve developer experience and learnability?
5. Can the decision be defended with benchmarks, tests, or documented reasoning?

If a proposal performs well on one axis but damages another, that tradeoff should be explicit rather than accidental.

## Relationship to Other Specs

This document sets direction.

The rest of the spec set explains how that direction is implemented:

- `error-system.md` defines the shared diagnostics model
- `parsing-profiles-and-language-modes.md` defines versioned and mode-driven parsing profiles
- `parser-core-semantics.md` defines parser commitment, choice, backtracking, and performance-safe execution semantics
- `parser-execution-and-consumption-models.md` defines tree, visitor, event, pull, and incremental parser surfaces
- `syntax-tree-and-materialization.md` defines the CST-first default tree shape, lowering strategy, and performance constraints
- `portable-grammar-ir-and-multi-target-backends.md` defines how one language definition may later feed Rust and non-Rust backends without forcing duplicate parser authoring
- `diagnostics-runtime.md` defines shared emission and collection behavior
- `source-spans-and-labels.md` defines source precision
- `recovery-and-error-boundaries.md` defines continue-on-error and resynchronization behavior
- `tokenizer-parser-integration-guidelines.md` defines best practices across the parsing pipeline
- `renderers-and-output.md` and `environment-detection.md` define output behavior

## Open Questions

1. Which benchmark suites should become the canonical performance gates for the project?
2. Which downstream use cases should define the minimum acceptable diagnostics UX?
3. How should performance, memory, and diagnostics quality regressions be tracked in CI over time?