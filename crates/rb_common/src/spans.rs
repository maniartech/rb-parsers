// ── SourceId ──────────────────────────────────────────────────────────────────

/// Opaque identifier for a source file or input buffer.
/// `SourceId(0)` is reserved as the "unknown / unset" sentinel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct SourceId(pub u32);

impl SourceId {
    pub const UNKNOWN: SourceId = SourceId(0);

    pub fn is_unknown(self) -> bool {
        self.0 == 0
    }
}

// ── SourcePosition ────────────────────────────────────────────────────────────

/// A byte position within a source buffer, together with its human-readable
/// line/column coordinates.
///
/// - `byte_offset`: 0-based byte index into the source string.
/// - `line`: 0-based line index (rendered as 1-based in output).
/// - `column`: 0-based UTF-8 character column (rendered as 1-based in output).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct SourcePosition {
    pub byte_offset: usize,
    pub line: usize,
    pub column: usize,
}

impl SourcePosition {
    pub const ZERO: SourcePosition = SourcePosition { byte_offset: 0, line: 0, column: 0 };

    pub fn new(byte_offset: usize, line: usize, column: usize) -> Self {
        SourcePosition { byte_offset, line, column }
    }

    /// Returns the 1-based line number for human display.
    pub fn display_line(self) -> usize {
        self.line + 1
    }

    /// Returns the 1-based column number for human display.
    pub fn display_column(self) -> usize {
        self.column + 1
    }
}

// ── SourceSpan ────────────────────────────────────────────────────────────────

/// A half-open byte range `[start, end)` anchored to a source file.
///
/// `end.byte_offset == start.byte_offset` represents an insertion point or
/// empty span.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct SourceSpan {
    pub source_id: SourceId,
    pub start: SourcePosition,
    pub end: SourcePosition,
}

impl SourceSpan {
    /// A zero-width span at offset zero in the unknown source. Used as a
    /// placeholder by scanners before the tokenizer loop sets the real span.
    pub const UNKNOWN: SourceSpan = SourceSpan {
        source_id: SourceId::UNKNOWN,
        start: SourcePosition::ZERO,
        end: SourcePosition::ZERO,
    };

    pub fn new(source_id: SourceId, start: SourcePosition, end: SourcePosition) -> Self {
        SourceSpan { source_id, start, end }
    }

    /// Creates a zero-width span (insertion point) at the given position.
    pub fn empty_at(source_id: SourceId, pos: SourcePosition) -> Self {
        SourceSpan { source_id, start: pos, end: pos }
    }

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

    /// Merges two spans sharing the same source, taking the outermost bounds.
    /// Returns `None` when sources differ.
    /// An UNKNOWN span is treated as identity: `merge(UNKNOWN, s) == s` and vice-versa.
    pub fn merge(self, other: SourceSpan) -> Option<SourceSpan> {
        if self == SourceSpan::UNKNOWN { return Some(other); }
        if other == SourceSpan::UNKNOWN { return Some(self); }
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
        Some(SourceSpan { source_id: self.source_id, start, end })
    }

    /// Returns the byte range as a `std::ops::Range<usize>`.
    pub fn byte_range(self) -> std::ops::Range<usize> {
        self.start.byte_offset..self.end.byte_offset
    }
}

// ── DiagnosticLocation ────────────────────────────────────────────────────────

/// Distinguishes real spans from synthetic positions used by parser recovery.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticLocation {
    /// A real byte range in source.
    Real(SourceSpan),
    /// A zero-width insertion point: where a token should have appeared.
    InsertionPoint { source_id: SourceId, at: SourcePosition },
    /// Points to the logical end of a source buffer.
    EndOfFile { source_id: SourceId, at: SourcePosition },
}

impl DiagnosticLocation {
    pub fn real(span: SourceSpan) -> Self {
        DiagnosticLocation::Real(span)
    }

    pub fn source_id(&self) -> SourceId {
        match self {
            DiagnosticLocation::Real(span) => span.source_id,
            DiagnosticLocation::InsertionPoint { source_id, .. } |
            DiagnosticLocation::EndOfFile { source_id, .. } => *source_id,
        }
    }

    pub fn start_position(&self) -> SourcePosition {
        match self {
            DiagnosticLocation::Real(span) => span.start,
            DiagnosticLocation::InsertionPoint { at, .. } |
            DiagnosticLocation::EndOfFile { at, .. } => *at,
        }
    }
}

impl From<SourceSpan> for DiagnosticLocation {
    fn from(s: SourceSpan) -> Self {
        DiagnosticLocation::Real(s)
    }
}

// ── LabelStyle ────────────────────────────────────────────────────────────────

/// Communicates the semantic role of a span label to renderers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelStyle {
    /// The precise failure site. Only one primary label per diagnostic.
    Primary,
    /// A structurally related location that gives context.
    Secondary,
    /// A recovery anchor or related token for additional orientation.
    Context,
}

// ── SpanLabel ─────────────────────────────────────────────────────────────────

/// A located, optionally annotated span.
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
        SpanLabel { style: LabelStyle::Primary, location, message: Some(message.into()) }
    }

    pub fn secondary(location: DiagnosticLocation) -> Self {
        SpanLabel { style: LabelStyle::Secondary, location, message: None }
    }

    pub fn secondary_with_message(location: DiagnosticLocation, message: impl Into<String>) -> Self {
        SpanLabel { style: LabelStyle::Secondary, location, message: Some(message.into()) }
    }

    pub fn context(location: DiagnosticLocation) -> Self {
        SpanLabel { style: LabelStyle::Context, location, message: None }
    }
}

// ── ScopeKind / ContextScope / DiagnosticContextRegion ───────────────────────

/// Semantic classification of a context region for diagnostics display.
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

/// A single scoped region that gives a diagnostic site its structural meaning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextScope {
    pub kind: ScopeKind,
    pub span: SourceSpan,
    /// Optional display label for the scope (e.g., "object at line 12").
    pub label: Option<String>,
}

/// Optional structured hierarchy for a diagnostic site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticContextRegion {
    /// The most immediate enclosing scope of the failure.
    pub owning_scope: Option<ContextScope>,
    /// Additional ancestors, ordered from innermost to outermost.
    pub ancestors: Vec<ContextScope>,
}

impl DiagnosticContextRegion {
    pub fn empty() -> Self {
        DiagnosticContextRegion { owning_scope: None, ancestors: Vec::new() }
    }

    pub fn has_context(&self) -> bool {
        self.owning_scope.is_some()
    }
}

// ── SnippetRequest ────────────────────────────────────────────────────────────

/// Passed by a renderer to a source store to extract display lines around a span.
#[derive(Debug, Clone)]
pub struct SnippetRequest {
    pub span: SourceSpan,
    /// How many lines of context to include before and after the span.
    pub context_lines: usize,
    /// Maximum renderable width in columns. `None` means no limit.
    pub max_width: Option<usize>,
}

impl SnippetRequest {
    pub fn new(span: SourceSpan) -> Self {
        SnippetRequest { span, context_lines: 1, max_width: None }
    }

    pub fn with_context(mut self, lines: usize) -> Self {
        self.context_lines = lines;
        self
    }

    pub fn with_max_width(mut self, width: usize) -> Self {
        self.max_width = Some(width);
        self
    }
}
