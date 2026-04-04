# visitor

`visitor` is reserved for AST traversal, analysis, and transform helpers that sit on top of parser-owned tree structures.

## Current Status

This crate is intentionally minimal while `rb_parser` is still defining its public AST and parser APIs.

## Intended Scope

When the parser API stabilizes, this crate should contain:

- tree traversal traits
- reusable visitor adapters
- analysis passes
- transformation utilities
- formatting or lint-style tree walkers

It should not duplicate tokenizer logic from `rb_tokenizer` or parser construction logic from `rb_parser`.