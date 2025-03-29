# another-crate Documentation

This crate serves as an example of a Rust library within a monorepo structure. It is designed to demonstrate how to organize and manage multiple related crates in a single repository.

## Purpose

The `another-crate` library provides additional functionality that complements the `rb_tokenizer` crate. It can be used independently or in conjunction with other crates in the monorepo.

## Usage

To use `another-crate`, add it as a dependency in your `Cargo.toml` file:

```toml
[dependencies]
another-crate = { path = "../another-crate" }
```

## Features

- Modular design that allows for easy integration with other crates.
- Well-defined public API for seamless usage.

## Examples

Here is a simple example of how to use `another-crate`:

```rust
use another_crate::some_function;

fn main() {
    some_function();
}
```

## Contributing

Contributions are welcome! Please open an issue or submit a pull request for any improvements or bug fixes.