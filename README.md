# rb-tokenizer

`rb-tokenizer` is a flexible, rule-based tokenizer written in Rust, designed to make text tokenization customizable and extendable. It supports a wide range of applications, from simple text parsing to complex programming language lexers.

## Features

- **Customizable Tokenization**: Easily define your own tokenization rules with regular expressions and symbols.
- **Extensible Architecture**: Add new rule types to suit your specific tokenization needs.
- **Performance**: Optimized for speed and efficiency, handling large texts swiftly.
- **Easy Integration**: Designed to be integrated into larger parsing or text analysis projects.
- **Configurable Behavior**: Control whitespace handling, error tolerance, position tracking, and more.
- **Robust Error Handling**: Configure how the tokenizer deals with unrecognized tokens.
- **Advanced Whitespace Management**: Properly handles whitespace in strings and other specialized tokens.

## Getting Started

### Prerequisites

Ensure you have Rust installed on your system. You can download Rust and `cargo` via [rustup](https://rustup.rs/).

### Installation

Add `rb-tokenizer` to your `Cargo.toml`:

```toml
[dependencies]
rb-tokenizer = { git = "https://github.com/maniartech/rb-tokenizer.git" }
```

### Basic Usage

To use `rb-tokenizer` in your project, start by creating a `Tokenizer` instance and adding rules:

```rust
use rb_tokenizer::{Tokenizer, TokenizerConfig};

// Create tokenizer with default configuration
let mut tokenizer = Tokenizer::new();

// Or with custom configuration
let config = TokenizerConfig {
    tokenize_whitespace: false,
    continue_on_error: true,
    error_tolerance_limit: 5,
    preserve_whitespace_in_tokens: true,
    track_token_positions: true,
};
let mut tokenizer = Tokenizer::with_config(config);

tokenizer.add_regex_rule(r"^\d+", "Number", None);
tokenizer.add_regex_rule(r"^[a-zA-Z_][a-zA-Z0-9_]*", "Identifier", None);
tokenizer.add_symbol_rule("(", "Operator", Some("OpenParen"));
tokenizer.add_symbol_rule(")", "Operator", Some("CloseParen"));
tokenizer.add_symbol_rule("+", "Operator", Some("Plus"));

let tokens = tokenizer.tokenize("ADD(2 + 2)").unwrap();
println!("{:?}", tokens);
```

### Configuration Options

The `TokenizerConfig` struct provides these configuration options:

- **tokenize_whitespace**: When `true`, whitespace characters are tokenized rather than skipped.
- **continue_on_error**: When `true`, the tokenizer will attempt to continue after encountering unrecognized tokens.
- **error_tolerance_limit**: Maximum number of errors before giving up tokenization.
- **preserve_whitespace_in_tokens**: Ensures whitespace is properly preserved in string literals and similar tokens.
- **track_token_positions**: When `true`, tracks and records line and column positions for each token.

```rust
// Modify configuration after creating tokenizer
*tokenizer.config_mut() = TokenizerConfig {
    tokenize_whitespace: true,
    ..tokenizer.config().clone()
};
```

## Rule Priority

Rules are processed in the order they are added, with earlier rules taking precedence. 
You can also add rules with explicit priority:

```rust
tokenizer.add_rule_with_priority(Box::new(your_rule), 0); // Highest priority (processed first)
```

## Whitespace Handling

The tokenizer has sophisticated whitespace handling capabilities:

- When `tokenize_whitespace` is `false`, whitespace is skipped during tokenization.
- When `tokenize_whitespace` is `true`, whitespace is treated as a separate token.
- `preserve_whitespace_in_tokens` ensures proper handling of whitespace within string literals and other token types where whitespace is significant.

This makes `rb-tokenizer` ideal for scenarios where precise whitespace control is needed, such as programming language lexers.

## Examples

You can find more examples in the `tests/` directory of the repository, demonstrating various use cases and configurations.

## Contributing

Contributions to `rb-tokenizer` are welcome! Here are a few ways you can help:

- **Reporting Issues**: Found a bug or have a feature request? Please open an issue.
- **Pull Requests**: Want to contribute code? Pull requests are warmly welcomed. Please ensure your code adheres to the project's coding standards and includes tests, if applicable.
- **Documentation**: Improvements to documentation or new examples are always appreciated.

Before contributing, please read our [CONTRIBUTING.md](CONTRIBUTING.md) guide.

## License

`rb-tokenizer` is distributed under the MIT License. See [LICENSE](LICENSE) for more information.

## Acknowledgments

- Inspired by the flexibility of rule-based tokenization in various programming languages and frameworks.
