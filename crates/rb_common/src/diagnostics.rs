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
    pub fn authored(text: impl Into<String>) -> Self {
        Hint { text: text.into(), is_auto: false }
    }

    pub fn auto_generated(text: impl Into<String>) -> Self {
        Hint { text: text.into(), is_auto: true }
    }
}

// Keep the old severity so that existing users of DiagnosticSeverity (e.g., rb_parser
// engine.rs which may still reference it) do not immediately break.  New code should
// use crate::catalog::ErrorSeverity instead.
#[deprecated(note = "Use rb_common::catalog::ErrorSeverity instead")]
pub use crate::catalog::ErrorSeverity as DiagnosticSeverity;


// ── Diagnostic ───────────────────────────────────────────────────────────────

/// The complete representation of one emitted diagnostic.
/// All sinks, renderers, and tooling consumers work with this type.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub code: ErrorCode,
    pub severity: ErrorSeverity,
    pub title: &'static str,
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
    pub fn is_error(&self) -> bool {
        self.severity == ErrorSeverity::Error
    }

    pub fn is_warning(&self) -> bool {
        self.severity == ErrorSeverity::Warning
    }

    pub fn primary_location(&self) -> Option<&DiagnosticLocation> {
        self.labels
            .iter()
            .find(|l| l.style == LabelStyle::Primary)
            .map(|l| &l.location)
    }

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

    pub fn primary(mut self, location: DiagnosticLocation) -> Self {
        self.inner.labels.insert(0, SpanLabel::primary(location));
        self
    }

    pub fn primary_labeled(mut self, location: DiagnosticLocation, msg: impl Into<String>) -> Self {
        self.inner.labels.insert(0, SpanLabel::primary_with_message(location, msg));
        self
    }

    pub fn secondary(mut self, location: DiagnosticLocation) -> Self {
        self.inner.labels.push(SpanLabel::secondary(location));
        self
    }

    pub fn secondary_labeled(mut self, location: DiagnosticLocation, msg: impl Into<String>) -> Self {
        self.inner.labels.push(SpanLabel::secondary_with_message(location, msg));
        self
    }

    pub fn context(mut self, region: DiagnosticContextRegion) -> Self {
        self.inner.context = Some(region);
        self
    }

    pub fn note(mut self, note: impl Into<String>) -> Self {
        self.inner.notes.push(note.into());
        self
    }

    pub fn hint(mut self, text: impl Into<String>) -> Self {
        self.inner.hints.push(Hint::authored(text));
        self
    }

    pub fn suggestion(mut self, suggestion: Suggestion) -> Self {
        self.inner.suggestions.push(suggestion);
        self
    }

    pub fn severity(mut self, severity: ErrorSeverity) -> Self {
        self.inner.severity = severity;
        self
    }

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
    fn emit(&self, diagnostic: &Diagnostic);
}

// ── NullSink ─────────────────────────────────────────────────────────────────

pub struct NullSink;

impl DiagnosticSink for NullSink {
    fn emit(&self, _diagnostic: &Diagnostic) {}
}

// ── CollectingSink ────────────────────────────────────────────────────────────

pub struct CollectingSink {
    collected: std::sync::Mutex<Vec<Diagnostic>>,
}

impl CollectingSink {
    pub fn new() -> Self {
        CollectingSink { collected: std::sync::Mutex::new(Vec::new()) }
    }

    pub fn take_all(&self) -> Vec<Diagnostic> {
        std::mem::take(&mut *self.collected.lock().unwrap())
    }

    pub fn snapshot(&self) -> Vec<Diagnostic> {
        self.collected.lock().unwrap().clone()
    }

    pub fn count(&self) -> usize {
        self.collected.lock().unwrap().len()
    }

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

pub struct HookSink<F: Fn(&Diagnostic) + Send + Sync> {
    hook: F,
}

impl<F: Fn(&Diagnostic) + Send + Sync> HookSink<F> {
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

pub struct CompositeSink {
    sinks: Vec<Box<dyn DiagnosticSink>>,
}

impl CompositeSink {
    pub fn new() -> Self {
        CompositeSink { sinks: Vec::new() }
    }

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

    pub fn has_errors(&self) -> bool { self.error_count > 0 }
    pub fn error_count(&self) -> usize { self.error_count }
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
    pub warnings_as_errors: bool,
    pub suppressed: Vec<ErrorCode>,
    pub overrides: Vec<(ErrorCode, ErrorSeverity)>,
}

impl SeverityPolicy {
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

