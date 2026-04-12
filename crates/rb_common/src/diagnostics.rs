use crate::catalog::{ErrorCatalog, ErrorCode, ErrorSeverity};
use crate::spans::{DiagnosticContextRegion, DiagnosticLocation, LabelStyle, SpanLabel};
use crate::suggestions::Suggestion;

// ── Hint ─────────────────────────────────────────────────────────────────────

/// A short, actionable prose string attached to a [`Diagnostic`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hint {
    /// The hint text. Must be specific and actionable.
    pub text: String,
    /// `true` when generated automatically by the framework;
    /// `false` when explicitly authored by a grammar/language author.
    pub is_auto: bool,
}

impl Hint {
    /// Creates a manually-authored hint with the given text.
    pub fn authored(text: impl Into<String>) -> Self {
        Hint { text: text.into(), is_auto: false }
    }

    /// Creates a framework-generated hint with the given text.
    pub fn auto_generated(text: impl Into<String>) -> Self {
        Hint { text: text.into(), is_auto: true }
    }
}

// Keep the old severity so that existing users of DiagnosticSeverity (e.g., rb_parser
// engine.rs which may still reference it) do not immediately break.  New code should
// use crate::catalog::ErrorSeverity instead.
#[deprecated(since = "0.2.0", note = "Use `rb_common::catalog::ErrorSeverity` instead")]
pub use crate::catalog::ErrorSeverity as DiagnosticSeverity;


// ── Diagnostic ───────────────────────────────────────────────────────────────

/// The complete representation of one emitted diagnostic.
/// All sinks, renderers, and tooling consumers work with this type.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// Machine-stable error code referencing an `ErrorCatalog` entry.
    pub code: ErrorCode,
    /// Severity level (error, warning, note, etc.).
    pub severity: ErrorSeverity,
    /// Short, static title as it appears in the catalog.
    pub title: &'static str,
    /// Rendered message with placeholders substituted.
    pub message: String,
    /// Primary failure site and any secondary/context labels.
    pub labels: Vec<SpanLabel>,
    /// Optional enclosing-scope hierarchy for context rendering.
    pub context: Option<DiagnosticContextRegion>,
    /// Additional factual notes (not actionable).
    pub notes: Vec<String>,
    /// Actionable hints.
    pub hints: Vec<Hint>,
    /// Structured fixes, ordered: primary first.
    pub suggestions: Vec<Suggestion>,
}

impl Diagnostic {
    /// Returns `true` if the severity is `Error`.
    pub fn is_error(&self) -> bool {
        self.severity == ErrorSeverity::Error
    }

    /// Returns `true` if the severity is `Warning`.
    pub fn is_warning(&self) -> bool {
        self.severity == ErrorSeverity::Warning
    }

    /// Returns the primary label's location, if one has been set.
    pub fn primary_location(&self) -> Option<&DiagnosticLocation> {
        self.labels
            .iter()
            .find(|l| l.style == LabelStyle::Primary)
            .map(|l| &l.location)
    }

    /// Returns `true` if at least one structured suggestion has been attached.
    pub fn has_suggestions(&self) -> bool {
        !self.suggestions.is_empty()
    }
}

// ── DiagnosticBuilder ────────────────────────────────────────────────────────

/// Ergonomic builder for constructing a `Diagnostic` from an error catalog entry.
pub struct DiagnosticBuilder {
    inner: Diagnostic,
}

impl DiagnosticBuilder {
    /// Starts a builder from a known template. `message` is the rendered message
    /// with placeholders already substituted.
    pub fn from_template(
        catalog: &dyn ErrorCatalog,
        code: ErrorCode,
        message: impl Into<String>,
    ) -> Self {
        let template = catalog.get(code).expect("unknown error code in catalog");
        DiagnosticBuilder {
            inner: Diagnostic {
                code,
                severity: template.severity,
                title: template.title,
                message: message.into(),
                labels: Vec::new(),
                context: None,
                notes: Vec::new(),
                hints: template.default_hints.iter().map(|h| Hint::authored(*h)).collect(),
                suggestions: Vec::new(),
            },
        }
    }

    /// Construct a diagnostic without a catalog (e.g., for testing or emergency use).
    pub fn new(
        code: ErrorCode,
        severity: ErrorSeverity,
        title: &'static str,
        message: impl Into<String>,
    ) -> Self {
        DiagnosticBuilder {
            inner: Diagnostic {
                code,
                severity,
                title,
                message: message.into(),
                labels: Vec::new(),
                context: None,
                notes: Vec::new(),
                hints: Vec::new(),
                suggestions: Vec::new(),
            },
        }
    }

    /// Attaches a primary label pointing at `location`.
    pub fn primary(mut self, location: DiagnosticLocation) -> Self {
        self.inner.labels.insert(0, SpanLabel::primary(location));
        self
    }

    /// Attaches a primary label with an annotation message.
    pub fn primary_labeled(mut self, location: DiagnosticLocation, msg: impl Into<String>) -> Self {
        self.inner.labels.insert(0, SpanLabel::primary_with_owned_message(location, msg.into()));
        self
    }

    /// Attaches a secondary (contextual) label.
    pub fn secondary(mut self, location: DiagnosticLocation) -> Self {
        self.inner.labels.push(SpanLabel::secondary(location));
        self
    }

    /// Attaches a secondary label with an annotation message.
    pub fn secondary_labeled(mut self, location: DiagnosticLocation, msg: impl Into<String>) -> Self {
        self.inner.labels.push(SpanLabel::secondary_with_owned_message(location, msg.into()));
        self
    }

    /// Attaches an enclosing-scope hierarchy for context rendering.
    pub fn context(mut self, region: DiagnosticContextRegion) -> Self {
        self.inner.context = Some(region);
        self
    }

    /// Appends an informational note (not actionable).
    pub fn note(mut self, note: impl Into<String>) -> Self {
        self.inner.notes.push(note.into());
        self
    }

    /// Appends an actionable hint.
    pub fn hint(mut self, text: impl Into<String>) -> Self {
        self.inner.hints.push(Hint::authored(text));
        self
    }

    /// Appends a structured suggestion.
    pub fn suggestion(mut self, suggestion: Suggestion) -> Self {
        self.inner.suggestions.push(suggestion);
        self
    }

    /// Overrides the severity inferred from the catalog template.
    pub fn severity(mut self, severity: ErrorSeverity) -> Self {
        self.inner.severity = severity;
        self
    }

    /// Consumes the builder and returns the completed `Diagnostic`.
    pub fn build(self) -> Diagnostic {
        self.inner
    }
}

// ── DiagnosticsMode ──────────────────────────────────────────────────────────

/// Controls whether a [`DiagnosticsContext`] collects, emits, both, or neither.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DiagnosticsMode {
    /// Ignore all diagnostics.
    Disabled,
    /// Store diagnostics internally for later inspection without any output.
    #[default]
    Collect,
    /// Emit through sinks immediately; do not retain.
    Emit,
    /// Both collect and emit.
    CollectAndEmit,
}

// ── DiagnosticSink trait ─────────────────────────────────────────────────────

/// The output interface. Each sink owns exactly one output path.
pub trait DiagnosticSink: Send + Sync {
    /// Emits `diagnostic` to this sink's output channel.
    fn emit(&self, diagnostic: &Diagnostic);
}

// ── NullSink ─────────────────────────────────────────────────────────────────

/// A sink that silently discards all diagnostics.
pub struct NullSink;

impl DiagnosticSink for NullSink {
    fn emit(&self, _diagnostic: &Diagnostic) {}
}

// ── CollectingSink ────────────────────────────────────────────────────────────

/// A thread-safe sink that buffers diagnostics in memory for later inspection.
pub struct CollectingSink {
    collected: std::sync::Mutex<Vec<Diagnostic>>,
}

impl CollectingSink {
    /// Creates a new empty collecting sink.
    pub fn new() -> Self {
        CollectingSink { collected: std::sync::Mutex::new(Vec::new()) }
    }

    /// Removes and returns all buffered diagnostics.
    pub fn take_all(&self) -> Vec<Diagnostic> {
        std::mem::take(&mut *self.collected.lock().unwrap())
    }

    /// Returns a clone of all buffered diagnostics without consuming them.
    pub fn snapshot(&self) -> Vec<Diagnostic> {
        self.collected.lock().unwrap().clone()
    }

    /// Returns the total number of buffered diagnostics.
    pub fn count(&self) -> usize {
        self.collected.lock().unwrap().len()
    }

    /// Returns the number of error-severity diagnostics buffered.
    pub fn error_count(&self) -> usize {
        self.collected.lock().unwrap().iter().filter(|d| d.is_error()).count()
    }
}

impl Default for CollectingSink {
    fn default() -> Self { Self::new() }
}

impl DiagnosticSink for CollectingSink {
    fn emit(&self, diagnostic: &Diagnostic) {
        self.collected.lock().unwrap().push(diagnostic.clone());
    }
}

// ── HookSink ─────────────────────────────────────────────────────────────────

/// A sink that calls a closure for every emitted diagnostic.
pub struct HookSink<F: Fn(&Diagnostic) + Send + Sync> {
    hook: F,
}

impl<F: Fn(&Diagnostic) + Send + Sync> HookSink<F> {
    /// Creates a new `HookSink` that calls `hook` on each diagnostic.
    pub fn new(hook: F) -> Self {
        HookSink { hook }
    }
}

impl<F: Fn(&Diagnostic) + Send + Sync> DiagnosticSink for HookSink<F> {
    fn emit(&self, diagnostic: &Diagnostic) {
        (self.hook)(diagnostic);
    }
}

// ── CompositeSink ─────────────────────────────────────────────────────────────

/// A sink that fans out each diagnostic to multiple child sinks.
pub struct CompositeSink {
    sinks: Vec<Box<dyn DiagnosticSink>>,
}

impl CompositeSink {
    /// Creates an empty `CompositeSink`.
    pub fn new() -> Self {
        CompositeSink { sinks: Vec::new() }
    }

    /// Appends `sink` and returns `self` (builder pattern).
    pub fn with(mut self, sink: Box<dyn DiagnosticSink>) -> Self {
        self.sinks.push(sink);
        self
    }
}

impl Default for CompositeSink {
    fn default() -> Self { Self::new() }
}

impl DiagnosticSink for CompositeSink {
    fn emit(&self, diagnostic: &Diagnostic) {
        for sink in &self.sinks {
            sink.emit(diagnostic);
        }
    }
}

// ── DiagnosticsContext ────────────────────────────────────────────────────────

/// The per-pipeline diagnostics state.
/// Passed as `&mut DiagnosticsContext` to tokenizer and parser internals.
pub struct DiagnosticsContext {
    mode: DiagnosticsMode,
    sink: Option<Box<dyn DiagnosticSink>>,
    collected: Vec<Diagnostic>,
    error_count: usize,
    warning_count: usize,
}

impl DiagnosticsContext {
    /// Creates a collecting-only context.
    pub fn new() -> Self {
        Self::collecting()
    }

    /// Creates a context that accumulates all diagnostics in memory.
    pub fn collecting() -> Self {
        DiagnosticsContext {
            mode: DiagnosticsMode::Collect,
            sink: None,
            collected: Vec::new(),
            error_count: 0,
            warning_count: 0,
        }
    }

    /// Creates a context that emits through the given sink and does not retain.
    pub fn emitting(sink: Box<dyn DiagnosticSink>) -> Self {
        DiagnosticsContext {
            mode: DiagnosticsMode::Emit,
            sink: Some(sink),
            collected: Vec::new(),
            error_count: 0,
            warning_count: 0,
        }
    }

    /// Creates a context that both collects and emits.
    pub fn collecting_and_emitting(sink: Box<dyn DiagnosticSink>) -> Self {
        DiagnosticsContext {
            mode: DiagnosticsMode::CollectAndEmit,
            sink: Some(sink),
            collected: Vec::new(),
            error_count: 0,
            warning_count: 0,
        }
    }

    /// Disabled context: ignores all diagnostics.
    pub fn null() -> Self {
        DiagnosticsContext {
            mode: DiagnosticsMode::Disabled,
            sink: None,
            collected: Vec::new(),
            error_count: 0,
            warning_count: 0,
        }
    }

    /// Emits one finalized diagnostic.
    pub fn emit(&mut self, diagnostic: Diagnostic) {
        if self.mode == DiagnosticsMode::Disabled {
            return;
        }
        match diagnostic.severity {
            ErrorSeverity::Error   => self.error_count += 1,
            ErrorSeverity::Warning => self.warning_count += 1,
            _ => {}
        }
        if matches!(self.mode, DiagnosticsMode::Collect | DiagnosticsMode::CollectAndEmit) {
            self.collected.push(diagnostic.clone());
        }
        if matches!(self.mode, DiagnosticsMode::Emit | DiagnosticsMode::CollectAndEmit) {
            if let Some(sink) = &self.sink {
                sink.emit(&diagnostic);
            }
        }
    }

    /// Returns `true` if at least one error-severity diagnostic has been emitted.
    pub fn has_errors(&self) -> bool { self.error_count > 0 }
    /// Returns the total count of error-severity diagnostics emitted.
    pub fn error_count(&self) -> usize { self.error_count }
    /// Returns the total count of warning-severity diagnostics emitted.
    pub fn warning_count(&self) -> usize { self.warning_count }

    /// Returns all collected diagnostics in emission order.
    pub fn collected(&self) -> &[Diagnostic] { &self.collected }

    /// Removes and returns all collected diagnostics.
    pub fn take_all(&mut self) -> Vec<Diagnostic> {
        std::mem::take(&mut self.collected)
    }

    /// Compat alias used in older tests and rb_parser.
    pub fn take(&mut self) -> Vec<Diagnostic> { self.take_all() }

    /// All diagnostics via `diagnostics()` accessor (compat for rb_parser engine).
    pub fn diagnostics(&self) -> &[Diagnostic] { &self.collected }

    /// Filters collected diagnostics for errors.
    pub fn errors(&self) -> impl Iterator<Item = &Diagnostic> {
        self.collected.iter().filter(|d| d.is_error())
    }

    /// Filters collected diagnostics by error code.
    pub fn by_code(&self, code: ErrorCode) -> impl Iterator<Item = &Diagnostic> {
        self.collected.iter().filter(move |d| d.code == code)
    }

    /// Returns `true` if this context is in collection mode.
    pub fn is_collecting(&self) -> bool {
        matches!(self.mode, DiagnosticsMode::Collect | DiagnosticsMode::CollectAndEmit)
    }
}

impl Default for DiagnosticsContext {
    fn default() -> Self { Self::collecting() }
}

// ── SeverityPolicy ───────────────────────────────────────────────────────────

/// Optional severity remapping and code suppression.
#[derive(Debug, Clone, Default)]
pub struct SeverityPolicy {
    /// Treat all warning-severity diagnostics as errors.
    pub warnings_as_errors: bool,
    /// Diagnostics with these codes are silently discarded.
    pub suppressed: Vec<ErrorCode>,
    /// Per-code severity overrides.
    pub overrides: Vec<(ErrorCode, ErrorSeverity)>,
}

impl SeverityPolicy {
    /// Applies the policy to `diagnostic`, returning `None` if it was suppressed.
    pub fn apply(&self, mut diagnostic: Diagnostic) -> Option<Diagnostic> {
        if self.suppressed.contains(&diagnostic.code) {
            return None;
        }
        if let Some((_, sev)) = self.overrides.iter().find(|(c, _)| *c == diagnostic.code) {
            diagnostic.severity = *sev;
        }
        if self.warnings_as_errors && diagnostic.severity == ErrorSeverity::Warning {
            diagnostic.severity = ErrorSeverity::Error;
        }
        Some(diagnostic)
    }
}

