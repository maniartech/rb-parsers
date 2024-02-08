# rb-tokenizer

`rb-tokenizer` is a flexible, rule-based tokenizer written in Rust, designed to make text tokenization customizable and extendable. It supports a wide range of applications, from simple text parsing to complex programming language lexers.

## Features

- **Customizable Tokenization**: Easily define your own tokenization rules with regular expressions and symbols.
- **Extensible Architecture**: Add new rule types to suit your specific tokenization needs.
- **Performance**: Optimized for speed and efficiency, handling large texts swiftly.
- **Easy Integration**: Designed to be integrated into larger parsing or text analysis projects.

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
use rb_tokenizer::{
    rules::{RegexRule, RuleType, SymbolRule},
    Tokenizer,
};

let mut tokenizer = Tokenizer::new();

tokenizer.add_rule(RuleType::Regex(RegexRule::new(r"^\d+", "Number", None)));
tokenizer.add_rule(RuleType::Regex(RegexRule::new(r"^[a-zA-Z_][a-zA-Z0-9_]*", "Identifier", None)));
tokenizer.add_rule(RuleType::Symbol(SymbolRule::new("+", "Operator", Some("Plus"))));
tokenizer.add_rule(RuleType::Symbol(SymbolRule::new("(", "Operator", Some("OpenParen"),)));
tokenizer.add_rule(RuleType::Symbol(SymbolRule::new(")", "Operator", Some("CloseParen"),)));

let tokens = tokenizer.tokenize("ADD(2 + 2)").unwrap();
println!("{:?}", tokens);
// Output:
// Ok([
//  Token { token_type: "Identifier", token_sub_type: None, value: "ADD", line: 1, column: 1 },
//  Token { token_type: "Operator", token_sub_type: Some("OpenParen"), value: "(", line: 1, column: 4 },
//  Token { token_type: "Number", token_sub_type: None, value: "2", line: 1, column: 5 },
//  Token { token_type: "Operator", token_sub_type: Some("Plus"), value: "+", line: 1, column: 7 },
//  Token { token_type: "Number", token_sub_type: None, value: "2", line: 1, column: 12 },
//  Token { token_type: "Operator", token_sub_type: Some("CloseParen"), value: ")", line: 1, column: 13 }
// ])

```

## Examples

You can find more examples in the `examples/` directory of the repository, demonstrating various use cases and configurations.

## Contributing

Contributions to `rb-tokenizer` are welcome! Here are a few ways you can help:

- **Reporting Issues**: Found a bug or have a feature request? Please open an issue.
- **Pull Requests**: Want to contribute code? Pull requests are warmly welcomed. Please ensure your code adheres to the project's coding standards and includes tests, if applicable.
- **Documentation**: Improvements to documentation or new examples are always appreciated.

Before contributing, please read our [CONTRIBUTING.md](CONTRIBUTING.md) guide.

## License

`rb-tokenizer` is distributed under the MIT License. See [LICENSE](LICENSE) for more information.

## Acknowledgments

- Thanks to all the contributors who spend their time to help improve this project.
- Inspired by the flexibility of rule-based tokenization in various programming languages and frameworks.
