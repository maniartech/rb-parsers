# Parser Core Semantics

## Objective

Define the engine-level meaning of grammar rules so one declarative grammar has predictable behavior across tree, event, pull, and future incremental surfaces while remaining competitive in performance and approachable for grammar authors.

This spec is about parser execution semantics, not syntax-tree representation or one specific grammar DSL.

## Why This Needs Its Own Spec

Without a parser-core semantics spec, the framework can look elegant while still becoming unpredictable in practice.

The project needs one place that answers questions such as:

- what ordered choice means
- when the parser is allowed to backtrack
- when a failure becomes committed
- how recovery interacts with commitment
- how ambiguity is resolved
- what happens with left recursion
- how performance expectations stay credible

Those are not implementation details. They determine whether the framework can be both fast and intuitive.

## Core Principle

The framework should default to deterministic, performance-safe parser semantics with explicit escape hatches for advanced cases.

That means:

- one grammar should mean the same thing across tree, event, pull, and incremental-oriented execution
- success, soft failure, committed failure, and recovery should have one coherent meaning
- advanced power should be additive rather than hidden behind surprising defaults
- the easy path should also be the high-confidence path for performance and diagnostics

## Product Stance: Easy Path and Fast Path Together

The framework should be approachable enough that a novice Rust programmer can build a serious parser without first learning parser-theory internals.

That requires more than friendly docs. It requires the engine semantics to reward straightforward grammar authoring.

In practice, that means:

- common combinators should carry strong default commitment and recovery behavior
- novice users should not need to understand memoization policy to avoid accidental slowdowns
- novice users should not need to scatter manual cuts through ordinary list, grouping, and expression grammars
- advanced users may still tune semantics deliberately when a grammar truly needs it

## Recommended Mental Model

Conceptually, a rule attempt should produce one of three outcomes:

```rust
pub enum ParseOutcome<T> {
    Success(T),
    SoftFailure(ParseFailure),
    CommittedFailure(ParseFailure),
}
```

The exact API may differ, but the behavior should match this model.

- `Success`: the rule matched and consumed input as defined by the grammar
- `SoftFailure`: the rule did not match and the caller may try another alternative without treating the branch as an error in itself
- `CommittedFailure`: the parser recognized enough structure that abandoning the branch would either hide the most useful diagnostic or permit harmful backtracking

Recovery may turn a committed failure into bounded continuation when configured to do so, but recovery should not retroactively make a different branch the correct parse.

## Ordered Choice Semantics

Ordered choice should be deterministic.

Recommended rules:

1. alternatives are tried from left to right
2. the next alternative is tried only after a soft failure
3. a committed failure stops the choice and becomes the branch result
4. diagnostics from losing soft-failure branches should normally be suppressed or used only for internal ranking
5. grammar authors should use precedence, guards, or explicit factoring when branch ordering alone would be unclear

This gives the framework a predictable author model and keeps branch behavior explainable.

## Commitment and Cut Semantics

The parser needs an explicit notion of commitment.

Recommended direction:

1. provide an explicit `cut()` or `commit()`-style combinator for expert use
2. allow high-level structural combinators to insert safe internal commitment points where the grammar shape is obvious
3. treat commitment as a parser-core event that influences both diagnostics and backtracking

Typical examples of safe commitment:

- after an opening delimiter and a required structural prefix have matched
- after a statement form has consumed a keyword that uniquely determines its branch
- inside precedence helpers once an operator sequence has already chosen the expression form

Once committed, downstream failures should normally be reported as errors in that branch rather than silently falling back to sibling alternatives.

## Built-In Combinator Expectations

The framework should make common grammar shapes safe by default.

At minimum, combinators such as these should carry intentional semantics:

- `between(...)`
- `list(...)`
- `repeat0(...)` and `repeat1(...)`
- delimiter-aware helpers
- `pratt(...)` or other precedence helpers

These should ideally provide:

- sensible internal commitment points
- well-defined recovery landmarks
- good diagnostics for missing separators or closing delimiters
- behavior that remains understandable without expert tuning

This is one of the main ways the framework can stay both fast and novice-friendly.

## Backtracking Policy

Unbounded speculative backtracking should not be the intended default behavior.

Recommended direction:

1. compile or interpret grammars so common cases behave linearly or near-linearly
2. prefer deterministic prefix handling and explicit commitment over deep speculative rewinds
3. warn or reject obviously expensive ambiguous prefixes when static analysis can detect them
4. keep any expert escape hatches explicit and off the starter path

The public story should not be "write anything and hope the runtime is fast enough".

## Left Recursion and Precedence

The first stable design should be conservative.

Recommended rules:

1. direct and indirect left recursion should be rejected during grammar compilation unless a dedicated facility handles it
2. expression grammars should use `pratt(...)` or precedence-specific helpers instead of relying on unsupported recursive patterns
3. compile-time diagnostics should explain why a left-recursive rule is unsupported and point authors toward the intended replacement

Future generalized support may be possible later, but it should not be an implicit promise of the initial model.

## Ambiguity Handling

The framework should be deterministic by default rather than acting like a generalized ambiguous parser.

Recommended stance:

1. grammar order, precedence, associativity, and guards define the chosen parse
2. the framework does not promise parse forests or multiple retained interpretations by default
3. if the grammar remains ambiguous in a way the runtime cannot resolve predictably, the framework should prefer compile-time rejection or clear diagnostics over hidden nondeterminism

That keeps both performance and mental models stronger.

## Memoization Policy

Parser semantics should not depend on whether memoization is enabled.

Recommended direction:

1. memoization is an implementation strategy, not the meaning of a rule
2. the runtime may apply selective memoization where it materially improves performance
3. full packrat-style memoization should not be the unqualified default if it causes avoidable memory costs
4. if memoization controls become public, they should be curated compile or runtime options, not a burden on the novice path

This preserves room for optimization without making authors reason about caches before they can write ordinary grammars.

## Recovery Interaction

Recovery semantics should layer on top of the core parser semantics rather than replacing them.

Recommended rules:

1. recovery should normally begin only from a committed failure or an explicitly recovery-aware combinator
2. recovery boundaries should not change which branch won the parse decision before the failure
3. recovered regions should remain visible to diagnostics and downstream consumers
4. recovery budgets should stay bounded and predictable

The goal is to collect more useful diagnostics, not to blur the core parse semantics.

## Complexity Expectations

Competitive performance requires explicit complexity expectations.

Baseline expectations:

1. well-authored common grammars should run in linear or near-linear time on representative inputs
2. the framework should document patterns that risk super-linear behavior
3. debug or instrumentation modes should expose useful counters such as rule invocations, backtracks, memo hits, and recovery steps
4. benchmark claims should use representative grammars rather than only toy inputs

## Diagnostics Expectations

The parser core should help grammar authors avoid bad designs early.

Useful compile-time diagnostics include:

- direct or indirect left recursion without a supported handler
- unreachable later alternatives because an earlier branch always commits first
- conflicting or impossible guards
- grammar shapes likely to cause harmful backtracking

Useful runtime diagnostics include:

- the committed context of a parse failure
- the recovery boundary that was chosen
- enough structural information to understand why a branch was selected

## Testing Guidance

Parser-core semantics should be tested with discipline.

Recommended coverage:

1. ordered choice tests that distinguish soft failure from committed failure
2. commitment tests around delimiters, keywords, and precedence helpers
3. grammar-compilation tests for unsupported left recursion
4. ambiguity tests that prove deterministic branch selection
5. recovery tests that verify recovery does not silently change prior branch decisions
6. performance regression tests for representative grammars with historically risky branch patterns

## Relationship to Other Specs

- `framework-objectives.md` defines the product requirement that the easy path must remain both credible and competitive
- `parser-execution-and-consumption-models.md` defines how one parser core feeds tree, event, pull, visitor, and incremental-oriented surfaces
- `syntax-tree-and-materialization.md` defines what the parser materializes by default once core semantics choose the structure
- `recovery-and-error-boundaries.md` defines how bounded continuation works after committed failure
- `parsing-profiles-and-language-modes.md` defines profile guards that may affect which rules are enabled

## Open Questions

1. Should explicit cut placement be part of the stable public grammar DSL immediately, or mostly an expert escape hatch at first?
2. How much automatic choice optimization or left factoring should grammar compilation perform before it becomes too magical?
3. Which memoization controls, if any, should be publicly exposed in the first implementation?
4. Which compile-time ambiguity checks are important enough to be on by default for novice users?