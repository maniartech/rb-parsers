use crate::diagnostics::{Diagnostic, Hint};
use crate::recovery::RecoveryAction;

// ── HintConfidence ────────────────────────────────────────────────────────────

/// Internal quality score for ranking hint candidates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HintConfidence {
    /// Cannot produce a useful hint; candidate should be discarded.
    TooLow,
    /// Broad advice; worth emitting only when nothing better exists.
    Low,
    /// Useful and reasonably specific.
    Medium,
    /// Specific, contextual, and actionable.
    High,
}

// ── HintOrigin ────────────────────────────────────────────────────────────────

/// Records where a hint candidate came from, for debugging and test assertions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HintOrigin {
    UserAuthored,
    TemplateDefault,
    SubsystemProvider(&'static str),
    CommonProvider(&'static str),
}

// ── HintCandidate ─────────────────────────────────────────────────────────────

/// A ranked hint candidate produced by one `HintProvider` invocation.
#[derive(Debug, Clone)]
pub struct HintCandidate {
    pub text: String,
    pub confidence: HintConfidence,
    pub origin: HintOrigin,
}

impl HintCandidate {
    pub fn into_hint(self) -> Hint {
        Hint {
            text: self.text,
            is_auto: !matches!(
                self.origin,
                HintOrigin::UserAuthored | HintOrigin::TemplateDefault
            ),
        }
    }

    pub fn is_strong(&self) -> bool {
        self.confidence >= HintConfidence::Medium
    }
}

// ── RecoveryInfo ──────────────────────────────────────────────────────────────

/// Structured recovery context attached to `HintContext` when a recovery action
/// was taken.
#[derive(Debug, Clone)]
pub struct RecoveryInfo {
    pub actions: Vec<RecoveryAction>,
}

// ── HintContext ───────────────────────────────────────────────────────────────

/// The read-only input to every `HintProvider`.
pub struct HintContext<'a> {
    pub diagnostic: &'a Diagnostic,
    /// Neighbouring tokens; format is crate-defined (`&[(TokenKind, SourceSpan)]` etc.)
    pub tokens: Option<&'a dyn std::any::Any>,
    pub recovery: Option<&'a RecoveryInfo>,
    /// Expected token string, when the error has a clear expectation.
    pub expected: Option<&'a str>,
    /// Found token string, when available.
    pub found: Option<&'a str>,
    /// The opening delimiter token when a closing is missing.
    pub opening_delimiter: Option<&'a str>,
}

impl<'a> HintContext<'a> {
    pub fn from_diagnostic(diagnostic: &'a Diagnostic) -> Self {
        HintContext {
            diagnostic,
            tokens: None,
            recovery: None,
            expected: None,
            found: None,
            opening_delimiter: None,
        }
    }

    pub fn with_expected_found(mut self, expected: &'a str, found: &'a str) -> Self {
        self.expected = Some(expected);
        self.found = Some(found);
        self
    }

    pub fn with_delimiter(mut self, opening: &'a str) -> Self {
        self.opening_delimiter = Some(opening);
        self
    }

    pub fn with_recovery(mut self, recovery: &'a RecoveryInfo) -> Self {
        self.recovery = Some(recovery);
        self
    }
}

// ── HintProvider trait ────────────────────────────────────────────────────────

pub trait HintProvider: Send + Sync {
    fn name(&self) -> &'static str;
    fn generate(&self, context: &HintContext<'_>) -> Vec<HintCandidate>;
}

// ── HintFilter ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct HintFilter {
    pub min_confidence: HintConfidence,
    pub max_hints: usize,
}

impl Default for HintFilter {
    fn default() -> Self {
        HintFilter { min_confidence: HintConfidence::Medium, max_hints: 2 }
    }
}

impl HintFilter {
    pub fn strict() -> Self {
        HintFilter { min_confidence: HintConfidence::High, max_hints: 1 }
    }
}

// ── HintPipeline ──────────────────────────────────────────────────────────────

pub struct HintPipeline {
    providers: Vec<Box<dyn HintProvider>>,
    filter: HintFilter,
}

impl HintPipeline {
    pub fn new(filter: HintFilter) -> Self {
        HintPipeline { providers: Vec::new(), filter }
    }

    pub fn with(mut self, provider: Box<dyn HintProvider>) -> Self {
        self.providers.push(provider);
        self
    }

    /// Generates automatic hints for `context`.
    /// Template and user-authored hints are NOT repeated here.
    pub fn generate(&self, context: &HintContext<'_>) -> Vec<Hint> {
        let mut candidates: Vec<HintCandidate> = self
            .providers
            .iter()
            .flat_map(|p| p.generate(context))
            .collect();

        candidates.sort_by(|a, b| b.confidence.cmp(&a.confidence));
        candidates.retain(|c| c.confidence >= self.filter.min_confidence);

        // Deduplicate near-identical hint texts (prefix match after trimming)
        let mut seen: Vec<String> = Vec::new();
        candidates.retain(|c| {
            let lower = c.text.to_lowercase();
            let prefix_len = lower.len().min(40);
            if seen.iter().any(|s| s.starts_with(&lower[..prefix_len])) {
                false
            } else {
                seen.push(lower);
                true
            }
        });

        candidates
            .into_iter()
            .take(self.filter.max_hints)
            .map(|c| c.into_hint())
            .collect()
    }
}

// ── Built-in common providers ─────────────────────────────────────────────────

/// Produces a hint when the error context includes both an expected and found token.
pub struct ExpectedVsFoundProvider;

impl HintProvider for ExpectedVsFoundProvider {
    fn name(&self) -> &'static str { "expected-vs-found" }

    fn generate(&self, context: &HintContext<'_>) -> Vec<HintCandidate> {
        match (context.expected, context.found) {
            (Some(exp), Some(found)) => vec![HintCandidate {
                text: format!(
                    "Expected `{exp}` but found `{found}`. Check for a typo or misplaced token."
                ),
                confidence: HintConfidence::Medium,
                origin: HintOrigin::CommonProvider("expected-vs-found"),
            }],
            (Some(exp), None) => vec![HintCandidate {
                text: format!("Expected `{exp}` here."),
                confidence: HintConfidence::Low,
                origin: HintOrigin::CommonProvider("expected-vs-found"),
            }],
            _ => vec![],
        }
    }
}

/// Produces a hint when an opening delimiter is known but unclosed.
pub struct DelimiterMismatchProvider;

impl HintProvider for DelimiterMismatchProvider {
    fn name(&self) -> &'static str { "delimiter-mismatch" }

    fn generate(&self, context: &HintContext<'_>) -> Vec<HintCandidate> {
        if let Some(opening) = context.opening_delimiter {
            let closing = match_closing(opening);
            let text = if let Some(close) = closing {
                format!("Add a closing `{close}` to match the opening `{opening}`.")
            } else {
                format!("The opening `{opening}` was never closed.")
            };
            return vec![HintCandidate {
                text,
                confidence: HintConfidence::High,
                origin: HintOrigin::CommonProvider("delimiter-mismatch"),
            }];
        }
        vec![]
    }
}

fn match_closing(opening: &str) -> Option<&'static str> {
    match opening {
        "(" => Some(")"),
        "[" => Some("]"),
        "{" => Some("}"),
        "<" => Some(">"),
        _ => None,
    }
}

// ── default_hint_pipeline ─────────────────────────────────────────────────────

/// Returns the standard pipeline wired with common providers.
/// Subsystem-specific providers are added by `rb_tokenizer` and `rb_parser`.
pub fn default_hint_pipeline() -> HintPipeline {
    HintPipeline::new(HintFilter::default())
        .with(Box::new(DelimiterMismatchProvider))
        .with(Box::new(ExpectedVsFoundProvider))
}
