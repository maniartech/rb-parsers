use rb_common::spans::DiagnosticLocation;
use rb_tokenizer::token_source::TokenSource;
use rb_tokenizer::tokens::Token;

// ── ParseFailure ──────────────────────────────────────────────────────────────

/// Carried in both failure variants of [`ParseOutcome`].
#[derive(Debug, Clone)]
pub struct ParseFailure {
    /// Source location where the failure occurred.
    pub location: DiagnosticLocation,
    /// Human-readable description of the token(s) that were expected.
    pub expected: Option<&'static str>,
    /// `true` after a `cut` combinator has been crossed — prevents backtracking.
    pub committed: bool,
    /// Nesting depth at the time of failure (used for error-message ranking).
    pub rule_depth: u32,
}

impl ParseFailure {
    /// Creates a soft (non-committed) failure.
    pub fn soft(location: DiagnosticLocation, expected: Option<&'static str>) -> Self {
        ParseFailure { location, expected, committed: false, rule_depth: 0 }
    }

    /// Creates a committed (non-backtrackable) failure.
    pub fn committed(location: DiagnosticLocation, expected: Option<&'static str>) -> Self {
        ParseFailure { location, expected, committed: true, rule_depth: 0 }
    }

    /// Attaches a rule-depth annotation to the failure.
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
    /// The rule matched and produced a value of type `T`.
    Success(T),
    /// The rule did not match; no input was consumed.
    SoftFailure(ParseFailure),
    /// The rule failed after passing a `cut` point; no backtracking allowed.
    CommittedFailure(ParseFailure),
}

impl<T> ParseOutcome<T> {
    /// Returns `true` if this is the `Success` variant.
    pub fn is_success(&self) -> bool { matches!(self, Self::Success(_)) }
    /// Returns `true` if this is the `SoftFailure` variant.
    pub fn is_soft_failure(&self) -> bool { matches!(self, Self::SoftFailure(_)) }
    /// Returns `true` if this is the `CommittedFailure` variant.
    pub fn is_committed_failure(&self) -> bool { matches!(self, Self::CommittedFailure(_)) }

    /// Unwraps the success value, panicking on any failure variant.
    pub fn unwrap(self) -> T {
        match self {
            Self::Success(v) => v,
            Self::SoftFailure(f) | Self::CommittedFailure(f) => {
                panic!("called unwrap() on a ParseOutcome failure: {:?}", f)
            }
        }
    }

    /// Converts to `Option<T>`, discarding any failure payload.
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

/// The action taken by the error-recovery strategy after a committed failure.
#[derive(Debug, Clone)]
pub enum RecoveryAction {
    /// Skipped tokens until a recovery landmark was found.
    SkipTo {
        /// The token type of the landmark that stopped the skip.
        landmark_token_type: &'static str,
        /// How many tokens were skipped to reach the landmark.
        skipped_count: usize,
    },
    /// Synthesised a missing token at the given position.
    InsertSynthetic {
        /// The token type of the synthesised token.
        token_type: &'static str,
        /// The source position where the synthetic token was inserted.
        at: rb_common::spans::SourcePosition,
    },
    /// Recovery was impossible; parsing halted at this position.
    Halted {
        /// The source position at which parsing stopped.
        at: rb_common::spans::SourcePosition,
    },
}

// ── ParseContext ──────────────────────────────────────────────────────────────

/// The mutable parse state threaded through every combinator call.
/// `!Send` — holds `&mut DiagnosticsContext`.
///
/// The type parameter `TS` is the concrete [`TokenSource`] implementation.
/// Using a generic parameter (rather than `Box<dyn TokenSource>`) lets the
/// compiler monomorphise `eval()` per token-source type, eliminating vtable
/// dispatch overhead on the `peek` / `advance` hot path.
///
/// The two common specialisations are:
/// - `ParseContext<'src, 'ctx, SliceTokenSource<'src>>` — zero-overhead slice mode.
/// - `ParseContext<'static, 'ctx, BufferedTokenSource<'static, I>>` — streaming mode.
pub struct ParseContext<'src, 'ctx, TS: TokenSource<'src>> {
    source: TS,
    /// The diagnostics accumulator for the current parse run.
    pub ctx: &'ctx mut rb_common::diagnostics::DiagnosticsContext,
    /// The language profile governing which rules are active.
    pub profile: &'ctx crate::profiles::ResolvedProfile,
    pub(crate) source_id: rb_common::spans::SourceId,
    committed_at: usize,
    pub(crate) rule_depth: u32,
    // Encodes the `'src` lifetime so the compiler can verify token lifetimes.
    _src: std::marker::PhantomData<&'src ()>,
}

impl<'src, 'ctx, TS: TokenSource<'src>> ParseContext<'src, 'ctx, TS> {
    /// Constructs a new `ParseContext` from a concrete [`TokenSource`] value,
    /// diagnostics context, resolved profile, and source identity.
    pub fn new(
        source: TS,
        ctx: &'ctx mut rb_common::diagnostics::DiagnosticsContext,
        profile: &'ctx crate::profiles::ResolvedProfile,
        source_id: rb_common::spans::SourceId,
    ) -> Self {
        ParseContext {
            source,
            ctx,
            profile,
            source_id,
            committed_at: 0,
            rule_depth: 0,
            _src: std::marker::PhantomData,
        }
    }

    /// Returns the token at the current cursor position without advancing.
    pub fn peek(&self) -> Option<&Token<'src>> {
        self.source.peek(0)
    }

    /// Returns the token `offset` positions ahead of the cursor without advancing.
    pub fn peek_ahead(&self, offset: usize) -> Option<&Token<'src>> {
        self.source.peek(offset)
    }

    /// Returns the token immediately before the current cursor, or `None`.
    ///
    /// Used by the engine to compute span ends for completed nodes.
    pub fn peek_back(&self) -> Option<&Token<'src>> {
        self.source.peek_back()
    }

    /// Consumes and returns the current token, advancing the cursor.
    pub fn advance(&mut self) -> Option<&Token<'src>> {
        self.source.advance()
    }

    /// Returns the current cursor index (token position).
    pub fn cursor(&self) -> usize { self.source.position() }

    /// Resets cursor to `pos`. Panics in debug builds if `pos < committed_at`.
    pub fn reset_to(&mut self, pos: usize) {
        debug_assert!(
            pos >= self.committed_at,
            "cannot backtrack across a committed point (committed_at={}, target={pos})",
            self.committed_at
        );
        self.source.reset_to(pos);
    }

    /// Records the current cursor as a commitment boundary.
    pub fn commit(&mut self) {
        let pos = self.source.position();
        if pos > self.committed_at {
            self.committed_at = pos;
            self.source.set_commit(pos);
        }
    }

    /// Returns `true` if any commitment boundary has been crossed.
    pub fn is_committed(&self) -> bool {
        self.committed_at > 0
    }

    /// Returns `true` if the cursor is past the last token.
    pub fn at_eof(&self) -> bool {
        self.source.is_exhausted()
    }

    /// Returns the position of the most recent commitment boundary.
    pub fn committed_at(&self) -> usize { self.committed_at }

    /// Current cursor position as a [`DiagnosticLocation`].
    pub fn location(&self) -> DiagnosticLocation {
        if let Some(tok) = self.source.peek(0) {
            DiagnosticLocation::real(tok.span)
        } else if let Some(tok) = self.source.last_token() {
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
}
