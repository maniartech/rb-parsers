# rb_tokenizer

`rb_tokenizer` is a flexible, rules-based tokenizer written in Rust, designed to make text tokenization customizable and extendable. It supports a wide range of applications, from simple text parsing to complex programming language lexers. You need to add custom scanning rules through various scanner types, such as regular expression and symbol scanners, to tokenize text into meaningful tokens.

## Features

- **Customizable Tokenization**: Easily define your own tokenization scanners with regular expressions and symbols.
- **Extensible Architecture**: Add new scanner types to suit your specific tokenization needs.
- **Performance**: Optimized for speed and efficiency, handling large texts swiftly.
- **Easy Integration**: Designed to be integrated into larger parsing or text analysis projects.
- **Configurable Behavior**: Control whitespace handling, error tolerance, position tracking, and more.
- **Robust Error Handling**: Configure how the tokenizer deals with unrecognized tokens.
- **Advanced Whitespace Management**: Properly handles whitespace in strings and other specialized tokens.
- **Flexible Escape Sequence Handling**: Comprehensive support for various escape sequence styles across different languages.

## Getting Started

### Prerequisites

Ensure you have Rust installed on your system. You can download Rust and `cargo` via [rustup](https://rustup.rs/).

### Installation

Add `rb_tokenizer` to your `Cargo.toml`:

```toml
[dependencies]
rb_tokenizer = { git = "https://github.com/maniartech/rb_tokenizer.git" }
```

### Basic Usage

To use `rb_tokenizer` in your project, start by creating a `Tokenizer` instance and adding scanners:

```rust
use rb_tokenizer::{Tokenizer, TokenizerConfig};

// Create tokenizer with default configuration
let mut tokenizer = Tokenizer::new();

// Or with custom configuration
let config = TokenizerConfig {
    tokenize_whitespace: false,
    continue_on_error: true,
    error_tolerance_limit: 5,
    track_token_positions: true,
};
let mut tokenizer = Tokenizer::with_config(config);

tokenizer.add_regex_scanner(r"^\d+", "Number", None);
tokenizer.add_regex_scanner(r"^[a-zA-Z_][a-zA-Z0-9_]*", "Identifier", None);
tokenizer.add_symbol_scanner("(", "Operator", Some("OpenParen"));
tokenizer.add_symbol_scanner(")", "Operator", Some("CloseParen"));
tokenizer.add_symbol_scanner("+", "Operator", Some("Plus"));

let tokens = tokenizer.tokenize("ADD(2 + 2)").unwrap();
println!("{:?}", tokens);
```

### Configuration Options

The `TokenizerConfig` struct provides these configuration options:

- **tokenize_whitespace**: When `true`, whitespace characters are tokenized rather than skipped.
- **continue_on_error**: When `true`, the tokenizer will attempt to continue after encountering unrecognized tokens.
- **error_tolerance_limit**: Maximum number of errors before giving up tokenization.
- **track_token_positions**: When `true`, tracks and records line and column positions for each token.

```rust
// Modify configuration after creating tokenizer
*tokenizer.config_mut() = TokenizerConfig {
    tokenize_whitespace: true,
    ..tokenizer.config().clone()
};
```

## Scanner Priority and Whitespace Handling

Scanners are scanned in the order they are added, with earlier scanners taking precedence.
You can also add scanners with explicit priority:

```rust
tokenizer.add_scanner_with_priority(Box::new(your_scanner), 0); // Highest priority (scanned first)
```

Each scanner is responsible for handling its own whitespace behavior. For example, string scanners should preserve their internal whitespace, while operator scanners typically don't need to handle whitespace:

```rust
// String scanner that preserves internal whitespace
tokenizer.add_regex_scanner(r#"^"([^"\\]|\\.)*""#, "String", None);

// Operator scanner that doesn't need to handle whitespace
tokenizer.add_symbol_scanner("+", "Operator", Some("Plus"));
```

## Whitespace Tokenization

The tokenizer provides two modes of whitespace handling:

- When `tokenize_whitespace` is `false`, whitespace is skipped during tokenization.
- When `tokenize_whitespace` is `true`, whitespace is treated as a separate token.

String literals and other tokens that need to preserve their internal whitespace handle this within their own scanner implementation, making the behavior consistent and predictable.

## Enhanced Escape Sequence Handling

The BlockScanner includes comprehensive support for handling escape sequences in various formats. This makes it easy to tokenize string literals and other content with complex escaping rules from different programming languages.

### Escape Rule Types

The BlockScanner supports four types of escape rules:

1. **Simple Escapes**: Traditional character escaping with a prefix like `\n` or `\t`
2. **Named Escapes**: Named sequences like HTML entities (`&lt;`, `&amp;`)
3. **Pattern Escapes**: Regular expression-based escapes like `\uXXXX` Unicode escapes
4. **Balanced Escapes**: Nested structures like `${...}` in template literals or `#{...}` in Ruby

### Using Escape Rules

You can configure a BlockScanner with custom escape rules:

```rust
let mut string_scanner = BlockScanner::new(
    "\"", "\"", "String", None, false, false, true
);

// Add a simple backslash escape
string_scanner.add_simple_escape('\\');

// Add HTML-style named entities
string_scanner.add_named_escape('&', ';', 10);

// Add Unicode escape sequence pattern
string_scanner.add_pattern_escape(r"\\u[0-9a-fA-F]{4}").unwrap();

// Add template expression escapes
string_scanner.add_balanced_escape("${", "}", true);

// Enable escape sequence transformation
string_scanner.set_transform_escapes(true);
string_scanner.add_escape_mapping("n", '\n');
string_scanner.add_escape_mapping("t", '\t');
string_scanner.add_escape_mapping("amp", '&');
```

This flexible system allows you to tokenize content from virtually any programming language or templating system with their unique escaping rules.

## Examples

You can find more examples in the `tests/` directory of the repository, demonstrating various use cases and configurations.

## Contributing

Contributions to `rb_tokenizer` are welcome! Here are a few ways you can help:

- **Reporting Issues**: Found a bug or have a feature request? Please open an issue.
- **Pull Requests**: Want to contribute code? Pull requests are warmly welcomed. Please ensure your code adheres to the project's coding standards and includes tests, if applicable.
- **Documentation**: Improvements to documentation or new examples are always appreciated.

Before contributing, please read our [CONTRIBUTING.md](CONTRIBUTING.md) guide.

## License

`rb_tokenizer` is distributed under the MIT License. See [LICENSE](LICENSE) for more information.

## Acknowledgments

- Inspired by the flexibility of scanner-based tokenization in various programming languages and frameworks.
