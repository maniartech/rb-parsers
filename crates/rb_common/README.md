# rb_common

`rb_common` is the shared infrastructure crate for the Rust Parsers workspace.

## Purpose

This crate is intended to hold cross-library primitives that should be shared by `rb_tokenizer`, `rb_parser`, and future workspace crates.

Current expected focus areas include:

- diagnostics and warning infrastructure
- structured error and reporting primitives
- source positions and spans
- common rendering or environment-detection helpers when they truly need to be shared

## Scope Rules

`rb_common` should stay narrowly scoped.

It should contain only shared building blocks that are genuinely reusable across multiple crates in the workspace. Tokenizer-only logic should remain in `rb_tokenizer`, and parser-only logic should remain in `rb_parser`.

## Specs

Design discussion documents for this crate live under `docs/specs/`.