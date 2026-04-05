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
8. Portable grammar IR and multi-target backend direction
9. Renderers for terminal, plain-text logs, and JSON output
10. Host environment detection such as color/no-color behavior

## Current Spec Set

1. [Framework Objectives](framework-objectives.md)
2. [Error System Draft](error-system.md)
3. [Parsing Profiles and Language Modes](parsing-profiles-and-language-modes.md)
4. [Parser Core Semantics](parser-core-semantics.md)
5. [Parser Execution and Consumption Models](parser-execution-and-consumption-models.md)
6. [Syntax Tree and Materialization](syntax-tree-and-materialization.md)
7. [Portable Grammar IR and Multi-Target Backends](portable-grammar-ir-and-multi-target-backends.md)
8. [Automatic Hinting](automatic-hinting.md)
9. [Source Spans and Labels](source-spans-and-labels.md)
10. [Recovery and Error Boundaries](recovery-and-error-boundaries.md)
11. [Suggestions and Fixes](suggestions-and-fixes.md)
12. [Diagnostics Runtime](diagnostics-runtime.md)
13. [Error Catalog and Compatibility](error-catalog-and-compatibility.md)
14. [Renderers and Output](renderers-and-output.md)
15. [Environment Detection](environment-detection.md)
16. [Tokenizer and Parser Integration Guidelines](tokenizer-parser-integration-guidelines.md)

## Working Rule

Specs in this directory should describe the problem, constraints, proposed API shape, and open questions before implementation is treated as stable.