# Fuzz testing

This directory contains [cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz)
targets for `rb_tokenizer` and `rb_parser`.

## Prerequisites

```sh
cargo install cargo-fuzz
rustup default nightly   # libFuzzer requires nightly
```

## Running a target

```sh
# From the workspace root:
cargo +nightly fuzz run tokenizer   -- -max_total_time=60
cargo +nightly fuzz run parser      -- -max_total_time=60
```

## Targets

| Target      | What it fuzzes |
|-------------|----------------|
| `tokenizer` | `Tokenizer::tokenize()` — feeds arbitrary UTF-8 to the tokenizer |
| `parser`    | Full pipeline — tokenises then parses with the JSON grammar |

## Investigating crashes

Crash inputs are saved in `fuzz/artifacts/<target>/crash-*`.
Replay a crash with:

```sh
cargo +nightly fuzz run tokenizer fuzz/artifacts/tokenizer/crash-<hash>
```
