# rb_common

`rb_common` is the shared infrastructure crate for the Rust Parsers workspace.

## Purpose

This crate is intended to hold cross-library primitives that should be shared by `rb_tokenizer`, `rb_parser`, and future workspace crates.

Current expected focus areas include:

- diagnostics and warning infrastructure
- structured error and reporting primitives
- parser-core semantics and performance-safe runtime contracts that must stay aligned across crates
- source positions and spans
- common rendering or environment-detection helpers when they truly need to be shared
- zero-config diagnostics UX with explicit overrides for advanced consumers
- novice-friendly defaults that can still scale into serious parser implementations

## Scope Rules

`rb_common` should stay narrowly scoped.

It should contain only shared building blocks that are genuinely reusable across multiple crates in the workspace. Tokenizer-only logic should remain in `rb_tokenizer`, and parser-only logic should remain in `rb_parser`.

## Specs

Design discussion documents for this crate live under `docs/specs/`.

The current spec set covers:

- framework-level objectives and quality goals
- shared diagnostics structure and template-backed error definitions
- automatic fallback hint generation and hint quality rules
- parsing profiles, language versions, strictness modes, and rule-based feature gating
- parser-core semantics such as ordered choice, commitment, backtracking, and performance-safe defaults
- parser execution and consumption models such as tree, visitor, event, pull, and incremental parsing
- syntax-tree representation, CST-first defaults, AST lowering, and performance-constrained materialization
- portable grammar IR and multi-target backend direction for reducing repeated DSL porting effort
- source spans, labels, hierarchical context regions, and parser-oriented location modeling
- structured suggestions and fixes
- diagnostics runtime and sink behavior
- bounded continue-on-error recovery and error boundary guidance
- error catalog stability and compatibility policy
- terminal, plain-text, and JSON rendering behavior with configurable selection and fallback
- environment and terminal capability detection with zero-config defaults and explicit overrides
- tokenizer-parser integration best practices and diagnostics usage guidance

These documents are design-stage references, not stable API guarantees yet. The index lives in [`docs/specs/README.md`](docs/specs/README.md).