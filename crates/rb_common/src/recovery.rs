use crate::spans::SourceSpan;

// ── RecoveryMode ──────────────────────────────────────────────────────────────

/// High-level recovery strategy selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RecoveryMode {
    /// Abort on the first error and return immediately.
    FailFast,
    /// Continue parsing up to configured limits when well-defined recovery
    /// boundaries exist.
    #[default]
    ContinueBounded,
}

// ── RecoveryConfig ────────────────────────────────────────────────────────────

/// Per-pipeline recovery tuning.
///
/// Defaults are chosen for interactive tooling and editor workflows.
#[derive(Debug, Clone)]
pub struct RecoveryConfig {
    pub mode: RecoveryMode,
    /// Maximum total error count before the pipeline stops attempting recovery.
    /// `0` means no limit (use with caution).
    pub max_errors: usize,
    /// Maximum number of tokens the parser may skip in a single recovery step.
    pub max_recovery_skips: usize,
}

impl Default for RecoveryConfig {
    fn default() -> Self {
        RecoveryConfig {
            mode: RecoveryMode::ContinueBounded,
            max_errors: 20,
            max_recovery_skips: 50,
        }
    }
}

impl RecoveryConfig {
    /// Returns a config suitable for strict batch validation.
    pub fn fail_fast() -> Self {
        RecoveryConfig {
            mode: RecoveryMode::FailFast,
            max_errors: 1,
            max_recovery_skips: 0,
        }
    }
}

// ── RecoveryBoundaryKind ──────────────────────────────────────────────────────

/// Structural category of a synchronization point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryBoundaryKind {
    /// Comma or similar item-separator token.
    Separator,
    /// Closing delimiter: `]`, `}`, `)`.
    ClosingDelimiter,
    /// Sentence or statement terminator: `;`, newline (when significant).
    StatementTerminator,
    /// Top-level block boundary: end of a named definition or top-level form.
    BlockBoundary,
    /// Language-defined custom boundary.
    Custom(&'static str),
}

// ── RecoveryBoundary ──────────────────────────────────────────────────────────

/// A single located boundary where the parser may safely resume.
#[derive(Debug, Clone)]
pub struct RecoveryBoundary {
    pub kind: RecoveryBoundaryKind,
    /// The span of the token or syntactic element that constitutes this
    /// boundary, if known.
    pub span: Option<SourceSpan>,
}

impl RecoveryBoundary {
    pub fn new(kind: RecoveryBoundaryKind) -> Self {
        RecoveryBoundary { kind, span: None }
    }

    pub fn with_span(kind: RecoveryBoundaryKind, span: SourceSpan) -> Self {
        RecoveryBoundary { kind, span: Some(span) }
    }
}

// ── RecoveryBoundarySet ───────────────────────────────────────────────────────

/// A named, composable set of boundary kinds. Grammar authors can start from
/// `common()` and extend.
#[derive(Debug, Clone, Default)]
pub struct RecoveryBoundarySet {
    pub kinds: Vec<RecoveryBoundaryKind>,
}

impl RecoveryBoundarySet {
    /// A sensible default for most collection-oriented languages: separators,
    /// closing delimiters, and statement terminators.
    pub fn common() -> Self {
        RecoveryBoundarySet {
            kinds: vec![
                RecoveryBoundaryKind::Separator,
                RecoveryBoundaryKind::ClosingDelimiter,
                RecoveryBoundaryKind::StatementTerminator,
            ],
        }
    }

    /// An aggressive set that includes block boundaries for top-level recovery.
    pub fn with_block_boundaries() -> Self {
        let mut s = Self::common();
        s.kinds.push(RecoveryBoundaryKind::BlockBoundary);
        s
    }

    pub fn with_kind(mut self, kind: RecoveryBoundaryKind) -> Self {
        self.kinds.push(kind);
        self
    }

    pub fn is_empty(&self) -> bool {
        self.kinds.is_empty()
    }
}

// ── RecoveryAction ────────────────────────────────────────────────────────────

/// Documents what the parser did during a recovery step, so it can be attached
/// to diagnostics and parse results.
#[derive(Debug, Clone)]
pub struct RecoveryAction {
    /// Human-readable description: "skipped 3 tokens to next `}`"
    pub description: String,
    /// The span of source that was skipped or mutated.
    pub skipped_span: Option<SourceSpan>,
    /// The recovery boundary the parser resumed at.
    pub resumed_at: Option<RecoveryBoundary>,
    /// Whether a synthetic token was inserted rather than real input consumed.
    pub was_synthetic_insertion: bool,
    /// Whether the parse results following this recovery should be considered
    /// fully trusted.
    pub confidence_degraded: bool,
}

// ── RecoveryState ─────────────────────────────────────────────────────────────

/// The outcome of a recovery attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryState {
    /// Parse succeeded without any recovery step.
    Clean,
    /// Parse succeeded after one or more bounded recovery steps.
    Recovered,
    /// Recovery was attempted but confidence in subsequent results is too low.
    PartialOnly,
    /// Recovery limit was hit. No further results are available.
    Exhausted,
}

impl RecoveryState {
    /// Returns true when results are safe to use for production purposes.
    pub fn is_usable(self) -> bool {
        matches!(self, RecoveryState::Clean | RecoveryState::Recovered)
    }
}

// ── RecoveredMarker<T> ────────────────────────────────────────────────────────

/// Wraps any parse result together with its recovery state and the actions
/// that produced it. This is the canonical "partial result" type.
#[derive(Debug, Clone)]
pub struct RecoveredMarker<T> {
    pub value: T,
    pub state: RecoveryState,
    pub actions: Vec<RecoveryAction>,
}

impl<T> RecoveredMarker<T> {
    pub fn clean(value: T) -> Self {
        RecoveredMarker { value, state: RecoveryState::Clean, actions: Vec::new() }
    }

    pub fn recovered(value: T, actions: Vec<RecoveryAction>) -> Self {
        RecoveredMarker { value, state: RecoveryState::Recovered, actions }
    }

    pub fn partial(value: T, actions: Vec<RecoveryAction>) -> Self {
        RecoveredMarker { value, state: RecoveryState::PartialOnly, actions }
    }

    pub fn is_clean(&self) -> bool {
        self.state == RecoveryState::Clean
    }

    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> RecoveredMarker<U> {
        RecoveredMarker { value: f(self.value), state: self.state, actions: self.actions }
    }
}

// ── ErrorBudget ───────────────────────────────────────────────────────────────

/// Per-layer error and skip counter. Each pipeline layer (tokenizer, parser)
/// maintains an independent budget and reports through a shared `DiagnosticsContext`.
#[derive(Debug, Clone, Default)]
pub struct ErrorBudget {
    pub error_count: usize,
    pub max_errors: usize,
    pub skip_count: usize,
    pub max_skips: usize,
}

impl ErrorBudget {
    pub fn from_config(config: &RecoveryConfig) -> Self {
        ErrorBudget {
            error_count: 0,
            max_errors: config.max_errors,
            skip_count: 0,
            max_skips: config.max_recovery_skips,
        }
    }

    /// Returns true when the budget allows another recovery attempt.
    pub fn can_recover(&self) -> bool {
        (self.max_errors == 0 || self.error_count < self.max_errors)
            && (self.max_skips == 0 || self.skip_count < self.max_skips)
    }

    pub fn record_error(&mut self) {
        self.error_count += 1;
    }

    pub fn record_skip(&mut self, count: usize) {
        self.skip_count += count;
    }
}
