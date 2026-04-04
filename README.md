# Rust Parsers Workspace

This repository is a Rust workspace for scanner-driven language tooling.

## Workspace Layout

```
rust-parsers
├── crates
│   ├── rb_common      # Shared cross-crate infrastructure and common abstractions
│   ├── rb_tokenizer   # Tokenization engine and scanner implementations
│   ├── rb_parser      # Parser crate under active design
│   └── visitor        # Visitor crate reserved for AST traversal utilities
├── tests              # Workspace-level integration and example coverage
├── Cargo.toml         # Workspace manifest
└── README.md          # Workspace overview
```

## Current Status

- `rb_tokenizer` is the primary implemented crate in the workspace.
- `rb_common` is the shared infrastructure crate for cross-library primitives.
- `rb_parser` is still in the API and architecture phase.
- `visitor` is reserved for traversal utilities and should stay minimal until parser-side AST types stabilize.

## Recommended Project Organization

The clean direction for the workspace is:

1. Keep cross-crate diagnostics, error/reporting primitives, source spans, and similar shared abstractions inside `rb_common`.
2. Keep tokenization logic, scanner primitives, and token models inside `rb_tokenizer`.
3. Keep grammar, parse errors, parse APIs, and language-specific parser examples inside `rb_parser`.
4. Keep AST walking, transforms, formatting passes, and analysis visitors inside `visitor`.
5. Keep workspace-level tests focused on end-to-end examples that cross crate boundaries.
6. Keep real-language examples executable and covered by tests, rather than storing example-only code paths that are not validated.

## Development Commands

Build the full workspace:

```bash
cargo build --workspace
```

Run the full test suite:

```bash
cargo test --workspace
```

Run lint checks:

```bash
cargo clippy --workspace --all-targets
```

## Quality Policy

Examples that model real languages must behave like supported language features, include regression coverage, and remain green in `cargo test --workspace`. Broken behavior should be fixed or clearly marked as unsupported, but never normalized by tests.

## License

This project is licensed under the MIT License. See the LICENSE file for details.