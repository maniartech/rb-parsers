# Rust Monorepo

This repository is a Rust monorepo that contains multiple crates, each serving different purposes. The main crate is `rb_tokenizer`, which provides functionality for tokenizing various input formats, including JSON. Additionally, there is another crate named `another-crate`, which serves as an example of how to structure additional functionality within the monorepo.

## Structure

The project is organized as follows:

```
rust-monorepo
├── crates
│   ├── rb_tokenizer       # The main tokenizer crate
│   │   ├── src            # Source code for rb_tokenizer
│   │   │   ├── lib.rs     # Library entry point
│   │   │   └── tokenizer.rs # Tokenization logic
│   │   ├── Cargo.toml     # Configuration for rb_tokenizer
│   │   └── README.md      # Documentation for rb_tokenizer
│   └── another-crate      # An additional example crate
│       ├── src            # Source code for another-crate
│       │   └── lib.rs     # Library entry point
│       ├── Cargo.toml     # Configuration for another-crate
│       └── README.md      # Documentation for another-crate
├── tests                  # Test files for the monorepo
│   ├── json_tests.rs      # Tests for JSON tokenizer functionality
│   └── integration_tests.rs # Integration tests for the monorepo
├── Cargo.toml             # Root configuration for the monorepo
└── README.md              # Overview of the monorepo
```

## Getting Started

To get started with this monorepo, clone the repository and navigate to the root directory. You can build and test each crate individually or run the tests for the entire workspace.

### Prerequisites

Make sure you have Rust and Cargo installed on your machine. You can install them from [the official Rust website](https://www.rust-lang.org/tools/install).

### Building the Crates

To build all the crates in the workspace, run:

```
cargo build
```

### Running Tests

To run the tests for all crates, use:

```
cargo test
```

## Contributing

Contributions are welcome! Please feel free to submit a pull request or open an issue if you find any bugs or have suggestions for improvements.

## License

This project is licensed under the MIT License. See the LICENSE file for more details.