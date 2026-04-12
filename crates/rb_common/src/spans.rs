// ── SourceId ──────────────────────────────────────────────────────────────────

/// Opaque identifier for a source file or input buffer.
/// `SourceId(0)` is reserved as the "unknown / unset" sentinel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SourceId(pub u32);

impl SourceId {
    /// The sentinel value for "no source file assigned" (byte 0 of source 0).
    pub const UNKNOWN: SourceId = SourceId(0);

    /// Returns `true` when this `SourceId` is the unknown sentinel.
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SourcePosition {
    /// 0-based byte offset into the source string.
    pub byte_offset: usize,
    /// 0-based line index (add 1 for user-facing display).
    pub line: usize,
    /// 0-based UTF-8 character column (add 1 for user-facing display).
    pub column: usize,
}

impl SourcePosition {
    /// A position at the very beginning of a source buffer.
    pub const ZERO: SourcePosition = SourcePosition { byte_offset: 0, line: 0, column: 0 };

    /// Constructs a `SourcePosition` from its parts.
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SourceSpan {
    /// The source file this span belongs to.
    pub source_id: SourceId,
    /// Inclusive start position.
    pub start: SourcePosition,
    /// Exclusive end position.
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

    /// Constructs a `SourceSpan` from its parts.
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
    InsertionPoint {
        /// The source file containing the insertion point.
        source_id: SourceId,
        /// The exact position of the insertion point.
        at: SourcePosition,
    },
    /// Points to the logical end of a source buffer.
    EndOfFile {
        /// The source file whose end is being referenced.
        source_id: SourceId,
        /// The position of the end-of-file marker.
        at: SourcePosition,
    },
}

impl DiagnosticLocation {
    /// Wraps a real `SourceSpan`.
    pub fn real(span: SourceSpan) -> Self {
        DiagnosticLocation::Real(span)
    }

    /// Returns the `SourceId` of the location.
    pub fn source_id(&self) -> SourceId {
        match self {
            DiagnosticLocation::Real(span) => span.source_id,
            DiagnosticLocation::InsertionPoint { source_id, .. } |
            DiagnosticLocation::EndOfFile { source_id, .. } => *source_id,
        }
    }

    /// Returns the source position at the start (or insertion point) of this location.
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
    /// How the label's span relates to the diagnostic (primary error site, secondary context, etc.).
    pub style: LabelStyle,
    /// The source location this label points at.
    pub location: DiagnosticLocation,
    /// Optional human-readable annotation for the span.
    ///
    /// Use `Cow::Borrowed("message")` for string literals (zero allocation).
    /// Use `Cow::Owned(format!(...))` for dynamic messages.
    pub message: Option<std::borrow::Cow<'static, str>>,
}

impl SpanLabel {
    /// Creates a primary `SpanLabel` with no annotation.
    pub fn primary(location: DiagnosticLocation) -> Self {
        SpanLabel { style: LabelStyle::Primary, location, message: None }
    }

    /// Creates a `SpanLabel` with a static annotation message and primary style.
    pub fn primary_with_message(location: DiagnosticLocation, message: &'static str) -> Self {
        SpanLabel { style: LabelStyle::Primary, location, message: Some(std::borrow::Cow::Borrowed(message)) }
    }

    /// Like [`primary_with_message`](Self::primary_with_message) but for dynamic strings.
    pub fn primary_with_owned_message(location: DiagnosticLocation, message: String) -> Self {
        SpanLabel { style: LabelStyle::Primary, location, message: Some(std::borrow::Cow::Owned(message)) }
    }

    /// Creates a secondary `SpanLabel` with no annotation.
    pub fn secondary(location: DiagnosticLocation) -> Self {
        SpanLabel { style: LabelStyle::Secondary, location, message: None }
    }

    /// Creates a secondary `SpanLabel` with a static annotation message.
    pub fn secondary_with_message(location: DiagnosticLocation, message: &'static str) -> Self {
        SpanLabel { style: LabelStyle::Secondary, location, message: Some(std::borrow::Cow::Borrowed(message)) }
    }

    /// Like [`secondary_with_message`](Self::secondary_with_message) but for dynamic strings.
    pub fn secondary_with_owned_message(location: DiagnosticLocation, message: String) -> Self {
        SpanLabel { style: LabelStyle::Secondary, location, message: Some(std::borrow::Cow::Owned(message)) }
    }

    /// Creates a context `SpanLabel` with no annotation.
    pub fn context(location: DiagnosticLocation) -> Self {
        SpanLabel { style: LabelStyle::Context, location, message: None }
    }
}

// ── ScopeKind / ContextScope / DiagnosticContextRegion ───────────────────────

/// Semantic classification of a context region for diagnostics display.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScopeKind {
    /// Entire source file.
    File,
    /// A JSON / YAML object or record.
    Object,
    /// A JSON / YAML array or list.
    Array,
    /// A curly-brace block or similar structural group.
    Block,
    /// A function or method call expression.
    Call,
    /// Any sub-expression context.
    Expression,
    /// A single statement.
    Statement,
    /// Language-defined custom scope. The string should be a short noun phrase.
    Custom(&'static str),
}

/// A single scoped region that gives a diagnostic site its structural meaning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextScope {
    /// The semantic category of this scope.
    pub kind: ScopeKind,
    /// The byte range covered by this scope.
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
    /// Constructs an empty region with no scope information.
    pub fn empty() -> Self {
        DiagnosticContextRegion { owning_scope: None, ancestors: Vec::new() }
    }

    /// Returns `true` if at least one enclosing scope has been recorded.
    pub fn has_context(&self) -> bool {
        self.owning_scope.is_some()
    }
}

// ── SnippetRequest ────────────────────────────────────────────────────────────

/// Passed by a renderer to a source store to extract display lines around a span.
#[derive(Debug, Clone)]
pub struct SnippetRequest {
    /// The span whose surrounding lines should be extracted.
    pub span: SourceSpan,
    /// How many lines of context to include before and after the span.
    pub context_lines: usize,
    /// Maximum renderable width in columns. `None` means no limit.
    pub max_width: Option<usize>,
}

impl SnippetRequest {
    /// Constructs a `SnippetRequest` with 1 line of context and no width cap.
    pub fn new(span: SourceSpan) -> Self {
        SnippetRequest { span, context_lines: 1, max_width: None }
    }

    /// Sets the number of context lines to include around the span.
    pub fn with_context(mut self, lines: usize) -> Self {
        self.context_lines = lines;
        self
    }

    /// Sets the maximum column width for the rendered snippet.
    pub fn with_max_width(mut self, width: usize) -> Self {
        self.max_width = Some(width);
        self
    }
}

// ── SourceRegistry ────────────────────────────────────────────────────────────

/// Metadata attached to a registered source buffer.
#[derive(Debug, Clone)]
pub struct SourceInfo {
    /// Optional file path (not set for in-memory / synthetic buffers).
    pub path:    Option<std::path::PathBuf>,
    /// Contents of the source buffer as owned text.
    pub content: String,
}

/// Thread-safe registry that allocates unique [`SourceId`]s and stores metadata
/// about each registered source buffer.
///
/// `SourceId(0)` is reserved as the `UNKNOWN` sentinel and is never returned by
/// [`register`](Self::register).
///
/// # Example
/// ```rust,ignore
/// let registry = SourceRegistry::new();
/// let id = registry.register(Some("foo.rb".into()), source_text.to_owned());
/// // id is guaranteed unique within this registry instance.
/// ```
pub struct SourceRegistry {
    next_id: std::sync::atomic::AtomicU32,
    files:   std::sync::Mutex<std::collections::HashMap<SourceId, SourceInfo>>,
}

impl SourceRegistry {
    /// Create a new, empty registry.
    pub fn new() -> Self {
        SourceRegistry {
            // Start at 1 so that 0 remains the UNKNOWN sentinel.
            next_id: std::sync::atomic::AtomicU32::new(1),
            files:   std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }

    /// Register a source buffer and return a unique [`SourceId`] for it.
    pub fn register(&self, path: Option<std::path::PathBuf>, content: String) -> SourceId {
        let id = SourceId(self.next_id.fetch_add(1, std::sync::atomic::Ordering::Relaxed));
        self.files
            .lock()
            .expect("SourceRegistry lock poisoned")
            .insert(id, SourceInfo { path, content });
        id
    }

    /// Look up the [`SourceInfo`] for a previously registered id.
    ///
    /// Returns `None` for `SourceId::UNKNOWN` or any id not registered in this
    /// instance.
    pub fn get(&self, id: SourceId) -> Option<SourceInfo> {
        self.files
            .lock()
            .expect("SourceRegistry lock poisoned")
            .get(&id)
            .cloned()
    }

    /// Returns `true` if `id` is registered in this registry.
    pub fn contains(&self, id: SourceId) -> bool {
        self.files
            .lock()
            .expect("SourceRegistry lock poisoned")
            .contains_key(&id)
    }

    /// Iterate over all registered (id, info) pairs.
    pub fn iter(&self) -> Vec<(SourceId, SourceInfo)> {
        self.files
            .lock()
            .expect("SourceRegistry lock poisoned")
            .iter()
            .map(|(&id, info)| (id, info.clone()))
            .collect()
    }
}

impl Default for SourceRegistry {
    fn default() -> Self {
        Self::new()
    }
}
