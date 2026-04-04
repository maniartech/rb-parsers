# Diagnostics Runtime

## Objective

Define how diagnostics are emitted, collected, ordered, deduplicated, and shared across tokenizer and parser instances.

This spec covers runtime behavior, not diagnostic content.

## Core Principle

Diagnostics must be instance-scoped, not process-global.

That means:

- one tokenizer can collect diagnostics silently
- another tokenizer can emit to a terminal sink
- a parser can share the same diagnostics context as the tokenizer that produced its tokens
- tests can inspect diagnostics deterministically without global side effects

## Core Types

Likely building blocks:

```rust
pub trait DiagnosticSink: Send + Sync {
    fn emit(&self, diagnostic: &Diagnostic);
}

pub enum DiagnosticsMode {
    Disabled,
    Collect,
    Emit,
    CollectAndEmit,
}
```

Potential runtime container:

```rust
pub struct DiagnosticsContext {
    // sink, storage, configuration, counters
}
```

## Requirements

1. Low-level components must not print directly.
2. Emission must go through a configured runtime context or sink.
3. Collection must be supported without terminal output.
4. Emission must be order-preserving.
5. The runtime must be safe to use in tests and concurrent scenarios.

## Standard Sink Modes

The runtime should support at least these behaviors:

- `NullSink`: ignore diagnostics
- `CollectingSink`: store diagnostics for later inspection
- `HookSink`: invoke user callback for each diagnostic
- `CompositeSink`: combine multiple sinks

## Tokenizer and Parser Sharing

The same context should be shareable across phases.

Example flow:

1. tokenizer emits warnings and recoverable errors
2. parser receives tokens and continues with the same diagnostics context
3. parser emits syntax diagnostics and recovery diagnostics into that same stream

This produces one coherent ordered diagnostic output for the full pipeline.

## Collection API

The runtime should support filtered access.

Examples:

- all diagnostics
- only errors
- only warnings
- by code
- by source subsystem

This matters because tokenizer today already exposes `last_errors`, and parser will likely need similar inspection helpers.

## Deduplication

Deduplication rules must be explicit.

Possible modes:

- no deduplication
- dedupe exact duplicates
- dedupe by code and span

Default behavior should probably avoid aggressive deduplication until real usage shows the need.

## Severity Policy

The runtime may later support policies such as:

- treat warnings as errors
- suppress selected codes
- downgrade selected severities

This is useful for CI, testing, and language-specific strictness modes.

## Open Questions

1. Should diagnostics context be mutable interior state or explicit return values?
2. Should sink hooks receive immutable diagnostics or a builder object before finalization?
3. Should ordering be strictly emission order or grouped by severity at render time only?
4. Should suppression and severity remapping belong to the runtime or a higher configuration layer?