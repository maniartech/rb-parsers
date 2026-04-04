# rb_common Specs

This directory is the design discussion area for `rb_common`.

The goal is to agree on shared cross-crate primitives before they are pushed into public APIs that `rb_tokenizer` and `rb_parser` will depend on.

## Initial Topics

1. Shared diagnostics and warning sink model
2. Structured error system with stable error codes, optional hints, and template-backed error catalogs
3. Source positions, spans, and file identifiers
4. Renderers for terminal, plain-text logs, and JSON output
5. Host environment detection such as color/no-color behavior

## Current Spec Set

1. [Error System Draft](error-system.md)
2. [Source Spans and Labels](source-spans-and-labels.md)
3. [Suggestions and Fixes](suggestions-and-fixes.md)
4. [Diagnostics Runtime](diagnostics-runtime.md)
5. [Error Catalog and Compatibility](error-catalog-and-compatibility.md)

## Working Rule

Specs in this directory should describe the problem, constraints, proposed API shape, and open questions before implementation is treated as stable.