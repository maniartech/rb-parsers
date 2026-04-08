# Spec: Source Spans and Labels

**Status**: Ready for implementation
**Module**: `rb_common::spans`
**Depends on**: nothing
**Requirement source**: `docs/requirements/source-spans-and-labels.md`

---

## Decisions Made

| Question | Decision |
|---|---|
| Storage byte offsets | 0-based internally; rendered as 1-based only in human output |
| Span end convention | Half-open: `[start_byte, end_byte)` throughout the codebase |
| Snippet extraction | Lives in `rb_common::spans` as a helper, used by renderers |
| Token ranges vs source spans | No separate token-range type initially; `SourceSpan` is the single vocabulary |

---

## Module Layout

```
rb_common::spans
├── SourceId
├── SourcePosition
├── SourceSpan
├── DiagnosticLocation
├── SpanLabel
├── LabelStyle
├── ScopeKind
├── ContextScope
├── DiagnosticContextRegion
└── SnippetRequest
```

---

## Types

### SourceId

Opaque identifier for a source file or input buffer. `u32` is sufficient for any
realistic workspace; a zero value is reserved as "no source".

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceId(pub u32);

impl SourceId {
    pub const UNKNOWN: SourceId = SourceId(0);

    pub fn is_unknown(self) -> bool {
        self.0 == 0
    }
}
```

---

### SourcePosition

A byte position within a source buffer together with its human-readable
line/column coordinates.

- `byte_offset`: 0-based byte index into the source string
- `line`: 0-based line index (rendered as 1-based in output)
- `column`: 0-based UTF-8 character column (rendered as 1-based in output)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct SourcePosition {
    pub byte_offset: usize,
    pub line: usize,
    pub column: usize,
}

impl SourcePosition {
    pub const ZERO: SourcePosition = SourcePosition {
        byte_offset: 0,
        line: 0,
        column: 0,
    };

    /// Returns the 1-based line number for human display.
    pub fn display_line(self) -> usize {
        self.line + 1
    }

    /// Returns the 1-based column number for human display.
    pub fn display_column(self) -> usize {
        self.column + 1
    }
}
```

---

### SourceSpan

A half-open byte range `[start, end)` anchored to a source file.

`end.byte_offset == start.byte_offset` represents an insertion point or empty span.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceSpan {
    pub source_id: SourceId,
    pub start: SourcePosition,
    pub end: SourcePosition,
}

impl SourceSpan {
    /// Returns true when start == end (zero-width insertion point or empty match).
    pub fn is_empty(self) -> bool {
        self.start.byte_offset == self.end.byte_offset
    }

    /// Returns the byte length of the span.
    pub fn byte_len(self) -> usize {
        self.end.byte_offset.saturating_sub(self.start.byte_offset)
    }

    /// Returns true when this span and `other` share the same source and overlap.
    pub fn overlaps(self, other: SourceSpan) -> bool {
        self.source_id == other.source_id
            && self.start.byte_offset < other.end.byte_offset
            && other.start.byte_offset < self.end.byte_offset
    }

    /// Merges two spans that share the same source, taking the outermost bounds.
    /// Returns `None` when sources differ.
    pub fn merge(self, other: SourceSpan) -> Option<SourceSpan> {
        if self.source_id != other.source_id {
            return None;
        }
        let start = if self.start.byte_offset <= other.start.byte_offset {
            self.start
        } else {
            other.start
        };
        let end = if self.end.byte_offset >= other.end.byte_offset {
            self.end
        } else {
            other.end
        };
        Some(SourceSpan {
            source_id: self.source_id,
            start,
            end,
        })
    }
}
```

---

### DiagnosticLocation

Distinguishes real spans from synthetic positions used by parser recovery.

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticLocation {
    /// A real byte range in source.
    Real(SourceSpan),

    /// A zero-width insertion point: where a token should have appeared.
    InsertionPoint {
        source_id: SourceId,
        at: SourcePosition,
    },

    /// Points to the logical end of a source buffer.
    EndOfFile {
        source_id: SourceId,
        at: SourcePosition,
    },
}

impl DiagnosticLocation {
    pub fn source_id(&self) -> SourceId {
        match self {
            DiagnosticLocation::Real(span) => span.source_id,
            DiagnosticLocation::InsertionPoint { source_id, .. } => *source_id,
            DiagnosticLocation::EndOfFile { source_id, .. } => *source_id,
        }
    }

    pub fn start_position(&self) -> SourcePosition {
        match self {
            DiagnosticLocation::Real(span) => span.start,
            DiagnosticLocation::InsertionPoint { at, .. } => *at,
            DiagnosticLocation::EndOfFile { at, .. } => *at,
        }
    }
}
```

---

### LabelStyle

Communicates the semantic role of a span label to renderers.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelStyle {
    /// The precise failure site. Only one primary label per diagnostic.
    Primary,
    /// A structurally related location that gives context (e.g., the opening
    /// delimiter for an unclosed block).
    Secondary,
    /// A recovery anchor or related token that provides additional orientation.
    Context,
}
```

---

### SpanLabel

A located, optionally annotated span.

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpanLabel {
    pub style: LabelStyle,
    pub location: DiagnosticLocation,
    pub message: Option<String>,
}

impl SpanLabel {
    pub fn primary(location: DiagnosticLocation) -> Self {
        SpanLabel { style: LabelStyle::Primary, location, message: None }
    }

    pub fn primary_with_message(location: DiagnosticLocation, message: impl Into<String>) -> Self {
        SpanLabel {
            style: LabelStyle::Primary,
            location,
            message: Some(message.into()),
        }
    }

    pub fn secondary(location: DiagnosticLocation) -> Self {
        SpanLabel { style: LabelStyle::Secondary, location, message: None }
    }

    pub fn secondary_with_message(location: DiagnosticLocation, message: impl Into<String>) -> Self {
        SpanLabel {
            style: LabelStyle::Secondary,
            location,
            message: Some(message.into()),
        }
    }
}
```

---

### ScopeKind

Semantic classification of a context region for diagnostics display.

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScopeKind {
    File,
    Object,
    Array,
    Block,
    Call,
    Expression,
    Statement,
    /// Language-defined custom scope. The string should be a short noun phrase.
    Custom(&'static str),
}
```

---

### ContextScope

A single scoped region that gives a diagnostic site its structural meaning.

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextScope {
    pub kind: ScopeKind,
    pub span: SourceSpan,
    /// Optional display label for the scope (e.g., "object at line 12").
    pub label: Option<String>,
}
```

---

### DiagnosticContextRegion

Optional structured hierarchy for a diagnostic site. The renderer chooses how
many ancestor levels to show based on available width and output format.

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticContextRegion {
    /// The most immediate enclosing scope of the failure.
    pub owning_scope: Option<ContextScope>,
    /// Additional ancestors, ordered from innermost to outermost.
    pub ancestors: Vec<ContextScope>,
}

impl DiagnosticContextRegion {
    pub fn empty() -> Self {
        DiagnosticContextRegion {
            owning_scope: None,
            ancestors: Vec::new(),
        }
    }

    pub fn has_context(&self) -> bool {
        self.owning_scope.is_some()
    }
}
```

---

### SnippetRequest

Passed by a renderer to a source store to extract display lines around a span.
This keeps snippet extraction deterministic and source-store-agnostic.

```rust
#[derive(Debug, Clone)]
pub struct SnippetRequest {
    pub span: SourceSpan,
    /// How many lines of context to include before and after the span.
    /// Default is 1.
    pub context_lines: usize,
    /// Maximum renderable width in columns. Used to decide whether to truncate
    /// long lines. `None` means no limit.
    pub max_width: Option<usize>,
}

impl SnippetRequest {
    pub fn new(span: SourceSpan) -> Self {
        SnippetRequest {
            span,
            context_lines: 1,
            max_width: None,
        }
    }
}
```

---

## Implementation Notes

- `SourceId(0)` is reserved; allocators should start at `1`.
- `SourcePosition` field order (`byte_offset`, `line`, `column`) matches Rust tokenizer
  token record conventions so bridging is zero-copy.
- `DiagnosticContextRegion` is optional cargo on a `Diagnostic`; parsers that do not
  track scopes simply omit it.
- No `SourceStore` trait is defined in this spec; that belongs in the renderer or
  diagnostics-runtime spec where it can be tied to a specific lifetime or
  ownership model.
# Source Spans and Labels

## Objective

Define the shared source location model for diagnostics emitted by tokenizer and parser code.

The design must support:

- precise byte-based tracking for Rust string slices
- line and column display for humans
- primary and secondary labels
- multi-span diagnostics
- hierarchical enclosing-region context
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
- `ContextScope`
- `DiagnosticContextRegion`

## Requirements

1. All spans must be anchored to a source identity, not just line and column.
2. Byte offsets must be first-class because tokenizer and parser both operate on slices and token ranges.
3. Line and column must remain first-class because diagnostics are shown to humans.
4. The model must support empty spans for insertion points and EOF positions.
5. The model must support synthetic spans for parser recovery and missing-token diagnostics.
6. The model must support multiple labels in a single diagnostic.
7. The model must support the immediate owning region for a diagnostic site.
8. The model must support relevant ancestor scopes so renderers can show hierarchical context when useful.

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

## Context Regions and Scope Hierarchies

Good parser diagnostics often need more than the failing token span.

Example:

- inside an object, parser expected `:` but found a string
- the primary location is the unexpected string
- the immediate owning region is the object span
- the parent region may be an array item or a larger enclosing object

The framework should be able to capture this structure explicitly so renderers can show the right surrounding region without guessing.

Likely shapes:

```rust
pub enum ScopeKind {
    File,
    Object,
    Array,
    Block,
    Call,
    Expression,
    Custom(&'static str),
}

pub struct ContextScope {
    pub kind: ScopeKind,
    pub span: SourceSpan,
    pub label: Option<String>,
}

pub struct DiagnosticContextRegion {
    pub focus: DiagnosticLocation,
    pub owning_scope: Option<ContextScope>,
    pub ancestors: Vec<ContextScope>,
}
```

Best practice is not to attach the full ancestor tree to every message indiscriminately. The model should allow it, and renderers should choose the most helpful level of detail.

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
5. renderers must be able to focus on the smallest useful region while still knowing the owning and ancestor scopes

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
- owning-scope and ancestor-scope context for nested structures

Typical parser-grade best practice is:

1. primary span points to the precise failure site
2. secondary labels point to structurally relevant anchors
3. enclosing region metadata identifies the smallest container that gives the failure meaning
4. ancestor scope metadata is available for advanced renderers and debugging tools

## Open Questions

1. Should `SourcePosition` be 1-based for display only, or 1-based in storage as well?
2. Should spans be half-open internally and converted only for rendering?
3. Should snippet extraction live in `rb_common` or in a separate rendering helper module?
4. Do we need a dedicated representation for token ranges in addition to source spans?
5. How many ancestor scopes should terminal renderers show by default before output becomes noisy?