# Source Spans and Labels

## Objective

Define the shared source location model for diagnostics emitted by tokenizer and parser code.

The design must support:

- precise byte-based tracking for Rust string slices
- line and column display for humans
- primary and secondary labels
- multi-span diagnostics
- synthetic or missing-token positions used by parser recovery
- snippet extraction for terminal and plain-text renderers

## Why This Needs Its Own Spec

Parser-grade diagnostics depend on source modeling more than almost any other part of the system. A good error catalog without a strong span model still produces weak diagnostics.

Examples that require this:

- "expected `)` to close this `(`"
- "string interpolation started here"
- "unexpected token after recovery point"
- "this delimiter closes the block opened here"

## Core Types

Likely building blocks:

```rust
pub struct SourceId(pub u32);

pub struct SourcePosition {
    pub byte_offset: usize,
    pub line: usize,
    pub column: usize,
}

pub struct SourceSpan {
    pub source_id: SourceId,
    pub start: SourcePosition,
    pub end: SourcePosition,
}
```

Additional types:

- `SpanLabel`
- `LabelStyle`
- `SnippetRequest`

## Requirements

1. All spans must be anchored to a source identity, not just line and column.
2. Byte offsets must be first-class because tokenizer and parser both operate on slices and token ranges.
3. Line and column must remain first-class because diagnostics are shown to humans.
4. The model must support empty spans for insertion points and EOF positions.
5. The model must support synthetic spans for parser recovery and missing-token diagnostics.
6. The model must support multiple labels in a single diagnostic.

## Labels

Diagnostics should distinguish between label roles.

Likely shape:

```rust
pub enum LabelStyle {
    Primary,
    Secondary,
    Context,
}

pub struct SpanLabel {
    pub style: LabelStyle,
    pub span: SourceSpan,
    pub message: Option<String>,
}
```

Usage examples:

- primary: the actual parse failure site
- secondary: the opening delimiter that explains the failure
- context: a recovery anchor or related token

## Synthetic and Missing Locations

Parser diagnostics often need to point at something that does not physically exist in the source.

Examples:

- missing `}` at end of file
- expected `,` between items
- inserted recovery token before the current token

The system should support this without abusing ordinary spans.

A likely extension:

```rust
pub enum DiagnosticLocation {
    Real(SourceSpan),
    InsertionPoint { source_id: SourceId, at: SourcePosition },
    EndOfFile { source_id: SourceId, at: SourcePosition },
}
```

## Snippet Extraction

Renderers should not guess how much source to show. The source model should make snippet extraction deterministic.

Requirements:

1. renderers must be able to request one or more source lines for a span
2. multiline spans must degrade gracefully in narrow terminals
3. snippet extraction must preserve tabs/newlines consistently
4. renderers must handle missing source text without crashing

## Tokenizer Integration

Tokenizer currently records line and column on tokens. The eventual shared model should allow tokenizer code to produce `SourceSpan` for:

- token extents
- unrecognized character positions
- unmatched delimiters
- regex normalization warnings if they need source context later

## Parser Integration

Parser will need more advanced cases:

- token-to-token spans
- missing token insertion points
- ambiguity regions
- precedence conflict explanations
- recovery windows that cover multiple consumed tokens

## Open Questions

1. Should `SourcePosition` be 1-based for display only, or 1-based in storage as well?
2. Should spans be half-open internally and converted only for rendering?
3. Should snippet extraction live in `rb_common` or in a separate rendering helper module?
4. Do we need a dedicated representation for token ranges in addition to source spans?