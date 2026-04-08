# Spec: Parser Engine Semantics

**Status**: Ready for implementation
**Module**: `rb_parser::engine`
**Depends on**: `rb_common::spans`, `rb_common::diagnostics`, `rb_parser::cst`, `rb_parser::profiles`
**Requirement source**: `docs/requirements/parser-core-semantics.md`

---

## Decisions Made

| Question | Decision |
|---|---|
| Parsing model | **PEG-style ordered choice** (`/`). The parser tries alternatives left-to-right and commits to the first that succeeds. |
| Backtracking | Soft (speculative) backtracking is available but **commitment cuts it off**. No unbounded global retry. |
| Left recursion | **Rejected at compile time.** `Grammar::compile()` runs a cycle check and returns `GrammarError::LeftRecursion` before any input is ever parsed. |
| Memoization | **Off by default.** Opt-in per grammar via `Grammar::compile_with_memo(keys)`. The default path is a simple recursive descent with commitment, which is faster than packrat for the grammars this framework targets. |
| Commitment rules | Four automatic commit points (see below); one manual `cut()` combinator. Commitment is one-way: once committed, a `SoftFailure` becomes a `CommittedFailure` and further backtracking is suppressed. |
| Error recovery | Recovery runs on `CommittedFailure` only. `SoftFailure` is local and silent; it triggers the next alternative in `one_of!`. |
| Diagnostics integration | The engine emits diagnostics through `DiagnosticsContext` at the point of commitment failure — not speculatively. |
| `ParseContext` thread safety | `ParseContext` is `!Send`; it holds a `&mut DiagnosticsContext`. Callers who need concurrent parsing must run independent parse sessions. |

---

## Core Outcome Type

`ParseOutcome<T>` is the return type of every combinator. The three variants
map to the three states a PEG parser can reach:

```rust
/// The outcome of running a grammar rule on a token stream.
#[derive(Debug)]
pub enum ParseOutcome<T> {
    /// The rule matched and produced a value.
    Success(T),

    /// The rule did not match. No input was consumed. The parser may
    /// backtrack and try the next alternative in `one_of!`.
    SoftFailure(ParseFailure),

    /// The rule started matching (commitment point was crossed) then failed.
    /// The parser does NOT backtrack. Recovery logic runs next.
    CommittedFailure(ParseFailure),
}

impl<T> ParseOutcome<T> {
    pub fn is_success(&self) -> bool { matches!(self, Self::Success(_)) }
    pub fn is_soft_failure(&self) -> bool { matches!(self, Self::SoftFailure(_)) }
    pub fn is_committed_failure(&self) -> bool { matches!(self, Self::CommittedFailure(_)) }

    /// Unwraps a `Success` value. Panics on failure variants.
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

    /// Escalates a `SoftFailure` to `CommittedFailure`.
    pub fn commit(self) -> Self {
        match self {
            Self::SoftFailure(f) => Self::CommittedFailure(f),
            other => other,
        }
    }
}
```

---

## ParseFailure

Carried in both failure variants.

```rust
use rb_common::spans::DiagnosticLocation;

#[derive(Debug, Clone)]
pub struct ParseFailure {
    /// Where the parse failed.
    pub location: DiagnosticLocation,
    /// What was expected at this location, if the combinator can name it.
    pub expected: Option<&'static str>,
    /// `true` when a commitment point was crossed before this failure.
    pub committed: bool,
    /// Depth of the rule stack at the point of failure (for diagnostics).
    pub rule_depth: u32,
}

impl ParseFailure {
    pub fn soft(location: DiagnosticLocation, expected: Option<&'static str>) -> Self {
        ParseFailure { location, expected, committed: false, rule_depth: 0 }
    }

    pub fn committed(location: DiagnosticLocation, expected: Option<&'static str>) -> Self {
        ParseFailure { location, expected, committed: true, rule_depth: 0 }
    }
}
```

---

## ParseContext

The mutable parse state threaded through every combinator call.

```rust
use rb_common::diagnostics::DiagnosticsContext;
use rb_parser::profiles::ResolvedProfile;
use rb_parser::recovery::RecoveryConfig;
use rb_tokenizer::tokens::TokenStream;

pub struct ParseContext<'src> {
    /// The token stream being parsed. Not owned here.
    pub(crate) stream:    &'src TokenStream<'src>,
    /// Live diagnostics context. Diagnostics are emitted at commitment failures.
    pub ctx:              &'src mut DiagnosticsContext,
    /// Recovery configuration resolved from the active profile.
    pub(crate) recovery:  RecoveryConfig,
    /// Active language profile — governs rule guards.
    pub profile:          &'src ResolvedProfile,
    /// Current cursor position (index into `stream`).
    cursor:               usize,
    /// The cursor position at the latest committed point, or 0.
    committed_at:         usize,
}

impl<'src> ParseContext<'src> {
    /// Returns the token at `cursor` without advancing.
    pub fn peek(&self) -> Option<&Token> {
        self.stream.get(self.cursor)
    }

    /// Returns the token `offset` positions ahead of `cursor`.
    pub fn peek_ahead(&self, offset: usize) -> Option<&Token> {
        self.stream.get(self.cursor + offset)
    }

    /// Advances `cursor` by one and returns the consumed token.
    pub fn advance(&mut self) -> Option<&Token> {
        let tok = self.stream.get(self.cursor);
        if tok.is_some() { self.cursor += 1; }
        tok
    }

    pub fn cursor(&self) -> usize { self.cursor }

    /// Resets `cursor` to `pos`. Only valid when `pos >= committed_at`.
    /// Panics in debug builds if called past the commitment boundary.
    pub fn reset_to(&mut self, pos: usize) {
        debug_assert!(
            pos >= self.committed_at,
            "cannot backtrack across a committed point (committed_at={}, target={pos})",
            self.committed_at
        );
        self.cursor = pos;
    }

    /// Records the current cursor as a commitment boundary. From this point
    /// forward `reset_to` will panic if given a position earlier than `cursor`.
    pub fn commit(&mut self) {
        self.committed_at = self.cursor;
    }

    pub fn is_committed(&self) -> bool {
        self.cursor > self.committed_at || self.committed_at > 0
    }

    pub fn at_eof(&self) -> bool {
        self.cursor >= self.stream.len()
    }
}
```

---

## Semantic Rules

### Ordered Choice

`one_of![a, b, c]` tries `a` first. If `a` returns `SoftFailure`, it tries
`b`. If `b` returns `SoftFailure`, it tries `c`. The first `Success` or
`CommittedFailure` stops the choice — `CommittedFailure` is never retried and
propagates upward.

This is identical to PEG's ordered choice `/` operator.

### Commitment Rules

Commitment transforms `SoftFailure` into `CommittedFailure` for all branches
that originate from the committed point forward. The four **automatic** commit
points are:

| Combinator | Commits after |
|---|---|
| `between(open, body, close)` | `open` matches (the delimiter is open; closing is mandatory) |
| `seq![a, b, c, ...]` | The first element `a` succeeds |
| `list(elem, sep)` | Each `sep` matches (another element is mandatory after a separator) |
| `pratt(atom)` | The first infix operator token is consumed |

The **manual** commit is:

| Construct | Effect |
|---|---|
| `cut()` combinator | Immediately marks the current position as committed; subsequent failures become `CommittedFailure` |

`cut()` is a first-class public DSL combinator, not an escape hatch for experts.
Grammar authors should use it wherever soft-backtracking semantics would produce
misleading error messages.

### Left Recursion

Left recursion is detected during `Grammar::compile()` via a depth-first cycle
check over the rule-reference graph. If a cycle is found, `compile()` returns
`Err(GrammarError::LeftRecursion { cycle })`. No runtime detection is performed.

Right recursion is permitted and supported. Indirect left recursion (A → B → A)
is also detected and rejected.

### Memoization

Off by default. The default execution strategy is an O(n) recursive descent
with commitment, which is optimal for the grammars this framework targets.

When memo is enabled via `Grammar::compile_with_memo(keys)`, the engine
caches `ParseOutcome<Option<CstNode>>` keyed by `(rule_id, cursor_position)`.
Only rules named in `keys` are memoized; this limits the overhead to rules
where ambiguity actually causes re-parsing.

**Warning**: Memoization is not compatible with diagnostic side effects inside
rules (e.g. rules that emit diagnostics as a side effect of matching). Any rule
that emits diagnostics must not be included in the memo keys.

### Recovery

Recovery logic runs when a `CommittedFailure` propagates to a rule that has
declared a `recover_to(landmarks)` boundary. The engine:

1. Emits a `RBP-*` diagnostic to `ctx` describing what was expected.
2. Advances the cursor to the next token matching a landmark in `landmarks`.
3. Resumes parsing from that position.

Recovery within a `between` combinator uses the closing delimiter as an
implicit landmark. Recovery within a `list` combinator uses the separator and
the enclosing `between`'s closing delimiter.

Rules that do not declare `recover_to` propagate `CommittedFailure` upward
until they reach a boundary or the top level.

---

## Error Emission Policy

The engine never emits a diagnostic for a `SoftFailure`. Diagnostics are
only emitted when:

1. A `CommittedFailure` reaches a `recover_to` boundary — the engine emits
   the primary diagnostic and applies recovery.
2. A `CommittedFailure` reaches the top level — the engine emits the primary
   diagnostic and the parse halts (or continues with a reduced result if the
   top-level combinator supports it).
3. `cut()` is followed by a terminal mismatch — the engine emits immediately.

This policy ensures each syntactic error produces exactly one diagnostic
(no cascading). The parser developer may add secondary labels and context to
the diagnostic builder.

---

## RecoveryConfig

```rust
/// Controls how aggressively the engine pursues recovery.
#[derive(Debug, Clone)]
pub struct RecoveryConfig {
    /// Maximum number of recovery steps before the engine gives up and halts.
    /// Default: 20. Set to 0 to disable recovery entirely.
    pub max_recovery_steps: usize,
    /// If `true`, the engine emits a `RBP-recovery-limit` warning when
    /// `max_recovery_steps` is reached.
    pub warn_on_limit: bool,
}

impl Default for RecoveryConfig {
    fn default() -> Self {
        RecoveryConfig { max_recovery_steps: 20, warn_on_limit: true }
    }
}
```

---

## Complexity Expectations

| Grammar class | Expected time complexity |
|---|---|
| LL(k) / LR(1)-like grammars (typical language grammars) | O(n) |
| Grammars with many alternatives but firm commitment | O(n · k) where k is max alternatives tried per position |
| Ambiguous grammars without commitment (anti-pattern) | O(exponential) — use `cut()` to prevent |
| Memoized grammars with true ambiguity | O(n²) worst case (packrat bound) |

Grammar authors are encouraged to run the included `Grammar::analyze_complexity()`
method during tests. It reports rules that could exhibit super-linear behavior
due to missing commitment.

---

## Implementation Notes

- `ParseContext` is a stack-allocated structure. Combinators receive it as
  `&mut ParseContext<'_>` and return `ParseOutcome<T>`.
- The engine does not use `Result<T, E>` because failure is not exceptional —
  it is a normal control-flow outcome in PEG parsing. `ParseOutcome<T>` models
  this three-way split explicitly.
- Combinators are free functions and macros, not methods on `ParseContext`,
  to keep the combinator vocabulary independent of the context implementation.
- The `TodoMemo` memo table will use `HashMap<(u32, usize), ParseOutcome<Option<CstNodeId>>>`
  keyed by `(rule_id_as_u32, cursor)`. Only opt-in rules are keyed.
