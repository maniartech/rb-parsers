# Spec: Automatic Hinting

**Status**: Ready for implementation
**Module**: `rb_common::hinting`
**Depends on**: `rb_common::diagnostics`, `rb_common::spans`, `rb_common::recovery`
**Requirement source**: `docs/requirements/automatic-hinting.md`

---

## Decisions Made

| Question | Decision |
|---|---|
| Hint confidence visibility | Internal ranking detail only. Public API exposes only the final ranked `Hint` values. |
| Multiple hints vs single hint | Cap at one primary automatic hint per provider. Multiple providers may contribute; the final ranked list caps at 2 hints total (primary + one supporting). |
| Common provider subsystem access | Common providers may inspect any field on `HintContext`; they must not call subsystem-specific APIs directly. |
| Experimental non-deterministic providers | Supported, but only behind `HintProviderFlags::experimental_only`. Never registered by default. |

---

## Module Layout

```
rb_common::hinting
├── HintConfidence
├── HintOrigin
├── HintCandidate
├── RecoveryInfo
├── HintContext
├── HintProvider (trait)
├── HintFilter
├── HintPipeline
├── ExpectedVsFound       (common built-in provider)
├── DelimiterMismatch     (common built-in provider)
└── default_hint_pipeline()
```

---

## Types

### HintConfidence

Internal quality score. Not exposed to callers of the final `Hint`.

```rust
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
```

---

### HintOrigin

Records where a hint candidate came from, for debugging and test assertions.

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HintOrigin {
    /// An explicit hint authored by the grammar or language author.
    UserAuthored,
    /// A default hint from the error template.
    TemplateDefault,
    /// A subsystem-specific provider (e.g. tokenizer scanner ordering).
    SubsystemProvider(&'static str),
    /// A framework-level common fallback provider.
    CommonProvider(&'static str),
}
```

---

### HintCandidate

A ranked hint candidate produced by one `HintProvider` invocation.

```rust
use crate::diagnostics::Hint;

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
            is_auto: !matches!(self.origin, HintOrigin::UserAuthored | HintOrigin::TemplateDefault),
        }
    }

    pub fn is_strong(&self) -> bool {
        self.confidence >= HintConfidence::Medium
    }
}
```

---

### RecoveryInfo

Structured recovery context attached to `HintContext` when a recovery action
was taken. Providers can use this to generate more precise hints.

```rust
use crate::recovery::RecoveryAction;

#[derive(Debug, Clone)]
pub struct RecoveryInfo {
    pub actions: Vec<RecoveryAction>,
}
```

---

### HintContext

The read-only input to every `HintProvider`. All fields are optional so the
same provider interface works for both tokenizer and parser diagnostics.

```rust
use crate::catalog::ErrorCode;
use crate::diagnostics::Diagnostic;
use crate::spans::SpanLabel;

pub struct HintContext<'a> {
    /// The diagnostic for which a hint is being generated.
    pub diagnostic: &'a Diagnostic,
    /// Optionally: neighbouring tokens with their kinds.
    /// Format is crate-defined; a simple `&[(TokenKind, SourceSpan)]` slice
    /// is sufficient for the initial implementation.
    pub tokens: Option<&'a dyn std::any::Any>,
    /// Structured recovery actions taken during parsing, if any.
    pub recovery: Option<&'a RecoveryInfo>,
    /// The string of the expected token, when the error has clear expectation.
    pub expected: Option<&'a str>,
    /// The string of the found token, when available.
    pub found: Option<&'a str>,
    /// Delimiter pairing context: the opening token string when a close is missing.
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
```

---

### HintProvider (trait)

```rust
pub trait HintProvider: Send + Sync {
    /// Provider name for tracing and test assertions.
    fn name(&self) -> &'static str;

    /// Generates zero or more ranked hint candidates.
    /// Returning an empty `Vec` is correct when context is insufficient.
    fn generate(&self, context: &HintContext<'_>) -> Vec<HintCandidate>;
}
```

---

### HintFilter

Policy for ranking and selecting the final hint list from all candidates.

```rust
#[derive(Debug, Clone)]
pub struct HintFilter {
    /// Minimum confidence to admit a hint.
    pub min_confidence: HintConfidence,
    /// Maximum number of automatic hints to attach to one diagnostic.
    pub max_hints: usize,
}

impl Default for HintFilter {
    fn default() -> Self {
        HintFilter {
            min_confidence: HintConfidence::Medium,
            max_hints: 2,
        }
    }
}

impl HintFilter {
    pub fn strict() -> Self {
        HintFilter { min_confidence: HintConfidence::High, max_hints: 1 }
    }
}
```

---

### HintPipeline

Runs all registered providers in order, collects candidates, applies the
filter, and returns the final `Vec<Hint>`.

The precedence from the requirement is preserved:

1. User-authored hints (already on the `Diagnostic`; not regenerated here)
2. Template default hints (already on the `Diagnostic`; not regenerated here)
3. Subsystem-specific providers (highest priority among auto providers)
4. Common fallback providers (lowest priority among auto providers)

```rust
use crate::diagnostics::Hint;

pub struct HintPipeline {
    providers: Vec<Box<dyn HintProvider>>,
    filter: HintFilter,
}

impl HintPipeline {
    pub fn new(filter: HintFilter) -> Self {
        HintPipeline { providers: Vec::new(), filter }
    }

    /// Registers a provider. Later-added providers run after earlier ones
    /// and have lower priority relative to the same confidence score.
    pub fn add(mut self, provider: Box<dyn HintProvider>) -> Self {
        self.providers.push(provider);
        self
    }

    /// Generates automatic hints for `context`.
    /// Template and user-authored hints are NOT repeated here; callers are
    /// responsible for merging the result with the diagnostic's existing hints.
    pub fn generate(&self, context: &HintContext<'_>) -> Vec<Hint> {
        let mut candidates: Vec<HintCandidate> = self
            .providers
            .iter()
            .flat_map(|p| p.generate(context))
            .collect();

        // Sort: highest confidence first, preserving stable provider order
        // within the same confidence tier.
        candidates.sort_by(|a, b| b.confidence.cmp(&a.confidence));

        // Apply minimum-confidence filter
        candidates.retain(|c| c.confidence >= self.filter.min_confidence);

        // Deduplicate near-identical hint texts (prefix match after trimming)
        let mut seen: Vec<String> = Vec::new();
        candidates.retain(|c| {
            let lower = c.text.to_lowercase();
            if seen.iter().any(|s| s.starts_with(&lower[..lower.len().min(40)])) {
                false
            } else {
                seen.push(lower);
                true
            }
        });

        // Cap count
        candidates
            .into_iter()
            .take(self.filter.max_hints)
            .map(|c| c.into_hint())
            .collect()
    }
}
```

---

### Built-in Common Providers

#### ExpectedVsFound

Produces a hint when the error context includes both an expected and found token.

```rust
pub struct ExpectedVsFoundProvider;

impl HintProvider for ExpectedVsFoundProvider {
    fn name(&self) -> &'static str {
        "expected-vs-found"
    }

    fn generate(&self, context: &HintContext<'_>) -> Vec<HintCandidate> {
        match (context.expected, context.found) {
            (Some(exp), Some(found)) => vec![HintCandidate {
                text: format!("Expected `{exp}` but found `{found}`. Check for a typo or misplaced token."),
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
```

#### DelimiterMismatch

Produces a hint when an opening delimiter is known but unclosed.

```rust
pub struct DelimiterMismatchProvider;

impl HintProvider for DelimiterMismatchProvider {
    fn name(&self) -> &'static str {
        "delimiter-mismatch"
    }

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
```

---

### default_hint_pipeline (free function)

Returns the standard pipeline wired with common providers. Subsystem-specific
providers are added by `rb_tokenizer` and `rb_parser` respectively.

```rust
pub fn default_hint_pipeline() -> HintPipeline {
    HintPipeline::new(HintFilter::default())
        .add(Box::new(DelimiterMismatchProvider))
        .add(Box::new(ExpectedVsFoundProvider))
}
```

---

## Integration Pattern

```rust
// In parser error-reporting code:
let context = HintContext::from_diagnostic(&diagnostic)
    .with_expected_found("}", &found_token)
    .with_delimiter("{");

let pipeline = default_hint_pipeline();
let auto_hints = pipeline.generate(&context);

// Append to any existing authored hints on the diagnostic
for hint in auto_hints {
    diagnostic.hints.push(hint);
}
```

---

## Implementation Notes

- `HintContext::tokens` uses `&dyn Any` so the hinting crate does not depend
  on specific token types from `rb_tokenizer`. Subsystem providers downcast to
  their own token slice type. Common providers never touch `tokens`.
- The deduplication in `HintPipeline::generate` is intentionally simple (40-char
  prefix). This is sufficient for the first implementation; more sophisticated
  similarity detection can replace it if needed.
- `HintConfidence::TooLow` candidates are effectively filtered by the default
  `HintFilter::min_confidence = Medium`. Returning `TooLow` is equivalent to
  returning an empty `Vec` but allows the pipeline to log provider output in
  debug builds without dropping the candidate silently.
- Tests for custom subsystem providers should use `HintFilter::strict()` with
  `max_hints: 1` to verify that the primary case works before testing fallback
  behavior.
# Automatic Hinting

## Objective

Define how the framework generates high-quality fallback hints when user-authored hints are not provided.

The goal is not to ensure every diagnostic has a hint. The goal is to ensure that any automatically generated hint feels specific, useful, and deliberate.

## Quality Bar

Automatic hints must not look like generic garbage.

That means:

- they must be tied to the actual failure context
- they must suggest a plausible next step
- they must add value beyond repeating the message
- they must avoid low-confidence speculation
- if the framework cannot produce a good hint, it should emit no hint

Silence is better than filler.

## Core Principle

Automatic hinting should be deterministic, structured, and testable.

The default framework system should not depend on opaque AI generation or vague prose templates. It should derive hints from diagnostic structure and subsystem-specific context.

Optional experimental providers could exist later, but the core built-in system should remain:

- reproducible
- benchmarkable
- snapshot-testable
- explainable to framework maintainers

## Hint Source Precedence

Recommended precedence order:

1. explicit user-authored hints
2. template default hints
3. subsystem-specific automatic hint providers
4. common fallback providers
5. no hint

This preserves author intent while still giving the framework room to help when no custom hint exists.

## Inputs to Hint Generation

Automatic hint providers should be able to use:

- error code and template metadata
- severity
- primary and secondary labels
- token spans and token kinds
- expected-versus-found token information
- parser recovery actions
- delimiter pairing information
- scanner configuration context when relevant

The more structured context a provider has, the better the hint can be.

## Likely Types

```rust
pub struct HintContext<'a> {
    pub diagnostic: &'a Diagnostic,
    pub source: Option<&'a SourceStore>,
    pub tokens: Option<&'a [Token]>,
    pub recovery: Option<&'a RecoveryInfo>,
}

pub struct HintCandidate {
    pub text: String,
    pub confidence: HintConfidence,
    pub origin: HintOrigin,
}

pub trait HintProvider: Send + Sync {
    fn generate(&self, context: &HintContext<'_>) -> Vec<HintCandidate>;
}
```

The exact API may differ, but the model should support ranking and quality filtering.

## Ranking and Filtering

The framework should rank candidates and drop weak ones.

Requirements:

1. candidates must be scored by confidence and specificity
2. the framework should prefer one strong hint over many weak ones
3. duplicated or near-duplicated hints should be collapsed
4. low-confidence hints should be filtered out by default

## Good Automatic Hint Characteristics

Strong automatic hints are:

- concrete: "Add `)` to close the call started here."
- contextual: mentions the actual token, delimiter, or structure involved
- actionable: tells the user what to try next
- proportionate: short enough to read quickly, precise enough to help

Weak automatic hints are:

- "Check the syntax near this token."
- "There might be an error in this expression."
- "Review your parser rules."

These should be treated as generation failures, not acceptable outputs.

## Tokenizer Use Cases

Tokenizer automatic hints should work well for cases such as:

- invalid regex pattern setup
- scanner precedence and ordering issues
- unmatched block delimiters
- invalid or unsupported escape forms
- unrecognized token where scanner registration is a likely cause

These hints should remain lexical and should not speculate about grammar.

## Parser Use Cases

Parser automatic hints should work well for cases such as:

- missing closing delimiters
- expected separator between items
- unexpected token after recovery point
- likely missing operator or operand
- known recovery insertions that map cleanly to user action

These hints should remain syntactic and should not pretend to know user intent when multiple interpretations are equally plausible.

## Relationship to Suggestions

Automatic hints are prose guidance.

They are distinct from suggestions:

- hint: tells the user what to consider doing
- suggestion: provides a structured edit or replacement

If the framework can produce a high-confidence edit, it may emit both a hint and a suggestion, but one should not be a weak textual copy of the other.

## Testing Guidance

Automatic hinting should be tested explicitly.

Tests should verify:

- a strong hint appears when context is sufficient
- no hint appears when context is too weak
- generic filler is not emitted
- ranked providers choose the better hint candidate
- subsystem-specific providers override weaker common fallbacks

## Open Questions

1. Should hint confidence be exposed publicly or remain an internal ranking detail?
2. Should the framework allow multiple automatic hints by default, or cap output at one primary hint?
3. How much subsystem-specific context should common hint providers be allowed to inspect?
4. Should experimental non-deterministic hint providers be supported behind explicit opt-in only?