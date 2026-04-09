use rb_common::spans::DiagnosticLocation;
use rb_tokenizer::tokens::Token;

// ── ParseFailure ──────────────────────────────────────────────────────────────

/// Carried in both failure variants of [`ParseOutcome`].
#[derive(Debug, Clone)]
pub struct ParseFailure {
    pub location: DiagnosticLocation,
    pub expected: Option<&'static str>,
    pub committed: bool,
    pub rule_depth: u32,
}

impl ParseFailure {
    pub fn soft(location: DiagnosticLocation, expected: Option<&'static str>) -> Self {
        ParseFailure { location, expected, committed: false, rule_depth: 0 }
    }

    pub fn committed(location: DiagnosticLocation, expected: Option<&'static str>) -> Self {
        ParseFailure { location, expected, committed: true, rule_depth: 0 }
    }

    pub fn with_depth(mut self, depth: u32) -> Self {
        self.rule_depth = depth;
        self
    }
}

// ── ParseOutcome ──────────────────────────────────────────────────────────────

/// The three-way result type returned by every combinator.
///
/// Matches PEG parser semantics:
/// - `Success` — rule matched, cursor advanced.
/// - `SoftFailure` — rule did not match, **no input consumed**, allows backtracking.
/// - `CommittedFailure` — rule started matching (commitment crossed) then failed;
///   NO backtracking, recovery logic runs.
#[derive(Debug)]
pub enum ParseOutcome<T> {
    Success(T),
    SoftFailure(ParseFailure),
    CommittedFailure(ParseFailure),
}

impl<T> ParseOutcome<T> {
    pub fn is_success(&self) -> bool { matches!(self, Self::Success(_)) }
    pub fn is_soft_failure(&self) -> bool { matches!(self, Self::SoftFailure(_)) }
    pub fn is_committed_failure(&self) -> bool { matches!(self, Self::CommittedFailure(_)) }

    pub fn unwrap(self) -> T {
        match self {
            Self::Success(v) => v,
            Self::SoftFailure(f) | Self::CommittedFailure(f) => {
                panic!("called unwrap() on a ParseOutcome failure: {:?}", f)
            }
        }
    }

    pub fn ok(self) -> Option<T> {
        match self { Self::Success(v) => Some(v), _ => None }
    }

    /// Escalates a `SoftFailure` into `CommittedFailure`. No-op on other variants.
    pub fn commit(self) -> Self {
        match self {
            Self::SoftFailure(f) => Self::CommittedFailure(f),
            other => other,
        }
    }

    /// Maps the success value, leaving failure variants unchanged.
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> ParseOutcome<U> {
        match self {
            Self::Success(v) => ParseOutcome::Success(f(v)),
            Self::SoftFailure(e) => ParseOutcome::SoftFailure(e),
            Self::CommittedFailure(e) => ParseOutcome::CommittedFailure(e),
        }
    }

    /// Returns the contained failure, or panics if success.
    pub fn failure(self) -> ParseFailure {
        match self {
            Self::SoftFailure(f) | Self::CommittedFailure(f) => f,
            Self::Success(_) => panic!("called failure() on a ParseOutcome::Success"),
        }
    }
}

// ── RecoveryAction ────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum RecoveryAction {
    SkipTo { landmark_token_type: &'static str, skipped_count: usize },
    InsertSynthetic { token_type: &'static str, at: rb_common::spans::SourcePosition },
    Halted { at: rb_common::spans::SourcePosition },
}

// ── ParseContext ──────────────────────────────────────────────────────────────

/// The mutable parse state threaded through every combinator call.
/// `!Send` — holds `&mut DiagnosticsContext`.
pub struct ParseContext<'src> {
    tokens: &'src [Token],
    pub ctx: &'src mut rb_common::diagnostics::DiagnosticsContext,
    #[allow(dead_code)] // stored for future use when ParseContext drives recovery directly
    pub(crate) recovery: crate::profiles::RecoveryConfig,
    pub profile: &'src crate::profiles::ResolvedProfile,
    pub(crate) source_id: rb_common::spans::SourceId,
    cursor: usize,
    committed_at: usize,
    pub(crate) rule_depth: u32,
}

impl<'src> ParseContext<'src> {
    pub fn new(
        tokens: &'src [Token],
        ctx: &'src mut rb_common::diagnostics::DiagnosticsContext,
        profile: &'src crate::profiles::ResolvedProfile,
        recovery: crate::profiles::RecoveryConfig,
        source_id: rb_common::spans::SourceId,
    ) -> Self {
        ParseContext {
            tokens,
            ctx,
            recovery,
            profile,
            source_id,
            cursor: 0,
            committed_at: 0,
            rule_depth: 0,
        }
    }

    pub fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.cursor)
    }

    pub fn peek_ahead(&self, offset: usize) -> Option<&Token> {
        self.tokens.get(self.cursor + offset)
    }

    pub fn advance(&mut self) -> Option<&Token> {
        let tok = self.tokens.get(self.cursor);
        if tok.is_some() { self.cursor += 1; }
        tok
    }

    pub fn cursor(&self) -> usize { self.cursor }

    /// Resets cursor to `pos`. Panics in debug builds if `pos < committed_at`.
    pub fn reset_to(&mut self, pos: usize) {
        debug_assert!(
            pos >= self.committed_at,
            "cannot backtrack across a committed point (committed_at={}, target={pos})",
            self.committed_at
        );
        self.cursor = pos;
    }

    /// Records the current cursor as a commitment boundary.
    pub fn commit(&mut self) {
        if self.cursor > self.committed_at {
            self.committed_at = self.cursor;
        }
    }

    pub fn is_committed(&self) -> bool {
        self.committed_at > 0
    }

    pub fn at_eof(&self) -> bool {
        self.cursor >= self.tokens.len()
    }

    pub fn committed_at(&self) -> usize { self.committed_at }

    /// Current cursor position as a [`DiagnosticLocation`].
    pub fn location(&self) -> DiagnosticLocation {
        if let Some(tok) = self.tokens.get(self.cursor) {
            DiagnosticLocation::real(tok.span)
        } else if let Some(tok) = self.tokens.last() {
            DiagnosticLocation::EndOfFile {
                source_id: self.source_id,
                at: tok.span.end,
            }
        } else {
            DiagnosticLocation::EndOfFile {
                source_id: self.source_id,
                at: rb_common::spans::SourcePosition::ZERO,
            }
        }
    }

    pub fn tokens(&self) -> &[Token] { self.tokens }
}
