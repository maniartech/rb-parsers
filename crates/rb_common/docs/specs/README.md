# rb_common Specs

This directory is the design discussion area for `rb_common`.

The goal is to agree on shared cross-crate primitives before they are pushed into public APIs that `rb_tokenizer` and `rb_parser` will depend on.

## Initial Topics

1. Shared diagnostics and warning sink model
2. Structured error system with stable error codes, optional hints, and template-backed error catalogs
3. Source positions, spans, and file identifiers
4. Parsing profiles, language versions, strictness modes, and feature gating
5. Parser core semantics such as choice, commitment, backtracking, and recovery interaction
6. Parser execution and consumption models
7. Syntax tree representation and materialization
8. Renderers for terminal, plain-text logs, and JSON output
9. Host environment detection such as color/no-color behavior

## Current Spec Set

1. [Framework Objectives](framework-objectives.md)
2. [Error System Draft](error-system.md)
3. [Parsing Profiles and Language Modes](parsing-profiles-and-language-modes.md)
4. [Parser Core Semantics](parser-core-semantics.md)
5. [Parser Execution and Consumption Models](parser-execution-and-consumption-models.md)
6. [Syntax Tree and Materialization](syntax-tree-and-materialization.md)
7. [Automatic Hinting](automatic-hinting.md)
8. [Source Spans and Labels](source-spans-and-labels.md)
9. [Recovery and Error Boundaries](recovery-and-error-boundaries.md)
10. [Suggestions and Fixes](suggestions-and-fixes.md)
11. [Diagnostics Runtime](diagnostics-runtime.md)
12. [Error Catalog and Compatibility](error-catalog-and-compatibility.md)
13. [Renderers and Output](renderers-and-output.md)
14. [Environment Detection](environment-detection.md)
15. [Tokenizer and Parser Integration Guidelines](tokenizer-parser-integration-guidelines.md)

## Working Rule

Specs in this directory should describe the problem, constraints, proposed API shape, and open questions before implementation is treated as stable.