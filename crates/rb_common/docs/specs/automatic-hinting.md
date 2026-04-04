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