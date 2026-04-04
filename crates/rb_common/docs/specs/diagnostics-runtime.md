# Diagnostics Runtime

## Objective

Define how diagnostics are emitted, collected, ordered, deduplicated, and shared across tokenizer and parser instances.

This spec covers runtime behavior, not diagnostic content.

Recovery and resume policy is defined in `recovery-and-error-boundaries.md`, but the runtime must be able to collect diagnostics emitted during those workflows coherently.

## Core Principle

Diagnostics must be instance-scoped, not process-global.

That means:

- one tokenizer can collect diagnostics silently
- another tokenizer can emit to a terminal sink
- a parser can share the same diagnostics context as the tokenizer that produced its tokens
- tests can inspect diagnostics deterministically without global side effects

The runtime should offer good defaults so most consumers can get high-quality diagnostics without assembling a complex sink graph by hand.

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

When multiple output renderers exist, the runtime should coordinate selection and fan-out rather than letting individual renderers race to claim the same output implicitly.

## Requirements

1. Low-level components must not print directly.
2. Emission must go through a configured runtime context or sink.
3. Collection must be supported without terminal output.
4. Emission must be order-preserving.
5. The runtime must be safe to use in tests and concurrent scenarios.
6. Diagnostics emitted during recovery must preserve the same ordering guarantees as diagnostics emitted before recovery.
7. If multiple output targets are configured, each target must have an explicit sink and renderer-selection path.
8. Common usage must be ergonomic with a minimal default runtime configuration.

## Standard Sink Modes

The runtime should support at least these behaviors:

- `NullSink`: ignore diagnostics
- `CollectingSink`: store diagnostics for later inspection
- `HookSink`: invoke user callback for each diagnostic
- `CompositeSink`: combine multiple sinks

For multi-renderer systems, `CompositeSink` is usually the right composition point. Each child sink can own one output target and select its renderer independently.

That composition should be easy to opt into, but not required for the simplest use cases.

## Tokenizer and Parser Sharing

The same context should be shareable across phases.

Example flow:

1. tokenizer emits warnings and recoverable errors
2. parser receives tokens and continues with the same diagnostics context
3. parser emits syntax diagnostics and recovery diagnostics into that same stream

This produces one coherent ordered diagnostic output for the full pipeline.

If the pipeline emits to multiple targets simultaneously, coherence should be preserved per target rather than by forcing one global renderer chain.

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

## Runtime Configuration Guidance

Best practice is a layered API:

1. a simple default context for normal library or CLI use
2. a small set of high-value options for common overrides
3. advanced sink and renderer composition for integrators

This keeps the end-user experience minimal while still giving framework users real control.

## Open Questions

1. Should diagnostics context be mutable interior state or explicit return values?
2. Should sink hooks receive immutable diagnostics or a builder object before finalization?
3. Should ordering be strictly emission order or grouped by severity at render time only?
4. Should suppression and severity remapping belong to the runtime or a higher configuration layer?