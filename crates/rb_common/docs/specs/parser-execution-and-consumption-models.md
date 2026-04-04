# Parser Execution and Consumption Models

## Objective

Define how `rb_parsers` should support different parser execution and consumption styles such as tree building, visitor-style traversal, event-based parsing, pull-based parsing, iterator-driven consumption, and incremental parsing.

This spec is about parser behavior surfaces and architectural layering, not one grammar DSL.

## Why This Needs Its Own Spec

These concerns are easy to mix together incorrectly.

For example:

- a visitor pattern is not the same thing as event-based parsing
- an iterator API is not automatically incremental
- a tree-based parser may still be built on top of an event-oriented core
- a pull parser may just be a wrapper over a push-style event stream

If the framework does not separate these ideas clearly, it will drift into an accidental API where each new use case adds another bespoke parser variant.

## Core Principle

The parser core and the parser consumption model should be separated.

That means:

- one grammar should not need to be rewritten for tree, event, or pull usage
- visitor traversal should generally operate on a produced syntax structure or structured event stream rather than defining the whole parser architecture
- incremental parsing should build on stable parser internals rather than replacing the public API with a completely different system

The easiest useful parser experience should remain the default, while advanced consumption models should be available through structured, rule-based choices rather than by forcing a different architecture on everyone.

Strategy-based design is a good fit for these advanced seams, but it should sit underneath a simple default API rather than replacing it.

## Recommended Overall Direction

Best practice for this ecosystem is:

1. keep the default public experience tree-oriented because it is easiest for most users
2. keep traversal and visitor-style APIs inside `rb_parser`, not in a separate crate for now
3. design the internal parse engine so it can drive multiple output surfaces
4. do not force all users into SAX-like or iterator-only parsing just to support advanced use cases
5. keep incremental parsing as a first-class architectural goal, even if the first implementation is phased in gradually
6. keep the advanced path aligned with the default path so users can adopt more power without learning a different parser model from scratch
7. use strategies for materialization, traversal, and advanced consumption where they create clean extension points

In short: tree-first default, multi-surface architecture underneath.

More specifically, the first stable tree surface should be a compact CST, with ASTs and other higher-level structures layered on top.

## Strategy-Based Design Guidance

Strategy-based design is likely the right approach for the unresolved areas such as:

- CST versus AST materialization
- visitor and walker behavior
- event versus tree consumers
- pull and iterator wrappers
- incremental reuse and invalidation policies

The important constraint is placement.

Best practice:

1. grammar authors define grammar rules once
2. parser core produces structured parse behavior once
3. strategy objects decide how that structure is materialized, traversed, or reused

That means strategy should usually govern:

- output materialization
- traversal style
- reuse policy
- consumer-facing adaptation

It should usually not govern:

- the meaning of grammar rules themselves
- basic parser correctness
- core diagnostic semantics

If strategy objects start changing what the grammar fundamentally means, the framework will become harder to reason about and test.

## Default API Versus Strategy API

The default user should not need to think about strategies.

Recommended model:

1. default parser methods choose sensible built-in strategies
2. named advanced surfaces may expose curated strategy choices
3. fully custom strategies are available only when the user needs deeper control

Conceptually:

```rust
let tree = parser.parse_tree(&tokens)?;
let ast = parser.parse_with_strategy(&tokens, AstLoweringStrategy::default())?;
let events = parser.parse_with_strategy(&tokens, EventStreamStrategy::default())?;
```

The exact API may differ, but the learning curve should stay layered.

## Separate Types Versus Strategies

The framework should be careful not to create a separate top-level parser type for every consumption style.

Usually not recommended:

- `AstParser`
- `EventParser`
- `PullParser`
- `VisitorParser`

Those names imply separate parser engines, separate grammar authoring flows, or duplicated parser configuration. In most cases that would make the framework harder to learn and harder to evolve.

Better defaults:

1. one core parser or compiled grammar type
2. normal methods for common surfaces
3. strategies for materialization or advanced consumption
4. a separate stateful runtime type only when lifecycle and caching truly differ

This usually means:

- AST is a materialization or lowering strategy, not a separate parser kind
- event and pull surfaces are alternate outputs or wrappers, not separate grammar systems
- visitors are traversal helpers over parse results, not parser types
- incremental parsing may justify a distinct runtime or session type because it carries state across edits

## Recommended Type Shape

Conceptually, the most likely healthy split is:

```rust
pub struct Parser {
    // compiled grammar, shared configuration
}

pub struct IncrementalParser {
    // parser core plus persistent reuse/cache state
}

pub trait ParseStrategy {
    type Output;

    fn consume(&mut self, event: ParseEvent);
    fn finish(self) -> Self::Output;
}
```

This shape keeps one coherent parser model while still allowing different outputs and long-lived incremental behavior.

## Why Incremental May Deserve Its Own Type

Incremental parsing is different from AST or visitor behavior.

It often needs:

- previous-tree or previous-event reuse state
- invalidation tracking
- edit application
- caching and identity stability

That is a real lifecycle difference, so a dedicated `IncrementalParser` or `IncrementalSession` can be justified.

By contrast, `AstParser` usually just means "parse and then materialize differently," which is better modeled as a strategy or lowering phase.

## Naming Guidance

Prefer names that describe lifecycle and responsibility clearly.

Good examples:

- `Parser`
- `IncrementalParser`
- `ParseStrategy`
- `AstLoweringStrategy`
- `EventStreamStrategy`
- `TreeVisitor`

Less desirable examples:

- `AstParser`
- `VisitorParser`
- `IteratorParser`

Those names blur the line between parsing, materialization, and traversal.

## Why Strategy Fits Here

The unresolved structure questions are not all-or-nothing choices.

For example:

- one grammar may feed a CST builder strategy
- the same parse engine may feed an AST-lowering strategy
- visitors may operate through a traversal strategy tuned for CST or AST
- incremental mode may use a reuse strategy constrained by stable identity rules

This is cleaner than hardcoding one global structure decision into the parser core too early.

## Risks of Overusing Strategy

Strategy-based design becomes harmful when:

- every ordinary parser setup requires choosing strategy types manually
- too many strategies overlap conceptually and differ only in small details
- strategy choice changes correctness instead of representation or consumption
- strategy configuration becomes a hidden second grammar DSL

The framework should avoid turning extensibility into ceremony.

## Key Distinctions

### Tree-Based Parsing

The parser produces a full syntax structure.

Recommended default for this framework:

- the first stable tree result should be a CST
- AST should be a lowering or materialization layer built on top of the CST

Strengths:

- easiest mental model for most users
- best fit for rich diagnostics and post-parse analysis
- natural fit for visitor and transform passes
- easiest API to document for common usage

Tradeoffs:

- more allocation than a pure streaming pipeline
- may be unnecessary for one-pass transformations or validation-only workflows

### Visitor Pattern

Visitor is primarily a traversal pattern, not a core parser execution mode.

Best practice:

- support visitor-style traversal on syntax trees or typed AST nodes
- optionally support event visitors or sinks for streaming consumers
- keep visitor APIs as a layer over parse results, not as the only way to consume parsing behavior

This is why keeping traversal in `rb_parser` is the right call for now.

### Event-Based Parsing

The parser emits structured events such as:

- start node
- finish node
- token accepted
- error emitted
- recovery action

Strengths:

- flexible foundation for multiple output models
- useful for streaming or low-allocation workflows
- can feed tree builders, event sinks, and pull wrappers
- often a strong basis for future incremental parsing

Tradeoffs:

- less ergonomic as the default API for ordinary users
- event contracts must be very carefully designed to preserve diagnostics and nesting clarity

### Pull-Based Parsing

The consumer requests the next event or next parsed item when ready.

Strengths:

- good for lazy consumers
- can wrap a push event core or incremental engine
- natural for iterator-style APIs

Tradeoffs:

- can complicate state management and error handling if made the core abstraction too early

### Iterator-Based Consumption

Iterator-style APIs are usually a surface form, not the fundamental parse model.

Examples:

- iterate over parse events
- iterate over top-level items in a file
- iterate over diagnostics or recovered subtrees

Best practice is to treat iterator APIs as ergonomic wrappers over a more explicit parser/event/tree model.

### Incremental Parsing

Incremental parsing matters for editors, language servers, and other interactive tooling.

Strengths:

- avoids full reparses after small edits
- enables responsive tooling for large files
- aligns well with long-lived diagnostics and syntax services

Tradeoffs:

- significantly more architectural complexity
- requires stable structure, reuse rules, and invalidation behavior
- must not compromise correctness or diagnostics trustworthiness

## Recommended Architecture Layers

The framework should aim for these conceptual layers:

1. token stream and source context
2. profile-aware parse engine
3. structured parse events and recovery metadata
4. materializers or consumers for specific output modes
5. traversal layers such as visitors, walkers, and iterators
6. optional incremental reuse and cache infrastructure

This layering keeps advanced use cases possible without making the common case harder.

## Internal Core Recommendation

The internal design should be event-capable, even if the main public API is tree-based.

That does not require the first public API to expose raw events directly, but it should mean:

- the parser can describe structure as explicit nested events or equivalent internal actions
- tree building is a consumer of that structured parse result rather than a baked-in side effect everywhere
- recovery actions and diagnostics can be attached to explicit parse boundaries
- later pull, streaming, or incremental surfaces do not require a total rewrite

This is the most defensible long-term direction for a framework that wants both developer ergonomics and serious tooling capability.

## Likely Types

```rust
pub enum ParseOutputMode {
    Tree,
    Events,
    Pull,
    Custom,
}

pub enum ParseEvent {
    StartNode { kind: SyntaxKind },
    FinishNode,
    Token { kind: TokenKind, span: SourceSpan },
    Error(Diagnostic),
    Recovery(RecoveryEvent),
}

pub trait ParseEventSink {
    fn push(&mut self, event: ParseEvent);
}

pub trait TreeVisitor {
    fn enter(&mut self, node: &SyntaxNode);
    fn leave(&mut self, node: &SyntaxNode);
}

pub struct IncrementalParseInput<'a> {
    pub previous_tree: Option<&'a SyntaxTree>,
    pub edits: &'a [TextEdit],
}
```

The exact API may differ, but the ecosystem should preserve this separation between parse engine, materialization, traversal, and incremental reuse.

## Default User Experience

The common-case experience should stay small.

Recommended default:

1. parse tokens into a syntax tree (CST) by default
2. traverse with simple helpers or visitors when needed
3. lower to typed AST only when the caller actually needs it
4. opt into lower-level event or pull surfaces only when the use case requires them

This keeps the framework usable for language authors who need correctness and diagnostics more than parser-mechanics control.

The fast path should be obvious, low-boilerplate, and well-documented. The advanced path should be discoverable as layered extensions, not as a separate expert-only subsystem.

Built-in strategies should back these defaults so users get the benefits of strategy-based architecture without paying its complexity cost up front.

The detailed representation and performance constraints for this default live in `syntax-tree-and-materialization.md`.

## Visitor Guidance

Visitor support should be present, but not overemphasized as the whole architecture.

Best practice:

- support tree walking helpers and visitor traits for AST and CST analysis
- keep traversal APIs composable with spans, diagnostics, and profile metadata
- avoid requiring every consumer to implement a visitor just to inspect parse results

Visitor is a post-parse and analysis convenience, not a substitute for the parser output model.

## Event and Pull Guidance

Event and pull surfaces should be treated as advanced but first-class options.

Good use cases:

- streaming validation of large inputs
- low-memory transformations
- building custom materializers
- editor or tooling pipelines that do not want a full user-facing AST at every stage

Best practice is to keep them aligned with the same diagnostics and recovery model as the tree-based surface.

## Incremental Guidance

Incremental parsing should be a design goal from the beginning, even if it is not the first fully implemented public feature.

To remain incremental-friendly, the framework should preserve:

- stable syntax kinds
- deterministic recovery boundaries
- explicit spans and source identity
- reusable tree or event segments where safe
- profile-aware invalidation, since a profile change may invalidate reuse assumptions

Incremental mode must never silently degrade diagnostics quality beyond acceptable limits.

## Diagnostics Expectations Across Models

All parser surfaces must preserve diagnostic quality.

Requirements:

1. errors and recovery actions must remain visible in tree, event, and pull workflows
2. hierarchical context should not disappear just because the consumer chose a non-tree surface
3. incremental reuse must not hide or duplicate diagnostics incorrectly
4. visitor and traversal helpers should preserve access to spans, source identity, and profile context

## Performance and Memory Guidance

The framework should not assume the same optimal surface for every workload.

General expectations:

- tree mode is often best for rich tooling and repeated analysis
- event mode can be better for streaming or custom sinks
- pull or iterator mode can improve ergonomics for lazy consumers
- incremental mode matters most for interactive repeated parses of changing text

The framework should make these tradeoffs explicit rather than pretending one model is always superior.

## Recommended Phasing

Reasonable implementation order:

1. one-shot tree-oriented parsing with strong diagnostics and recovery
2. traversal and visitor helpers over produced structures
3. explicit event surface or internal event capability
4. pull or iterator wrappers over event streams where justified
5. incremental parsing once the tree or event model is stable enough to reuse safely

This gives the project a credible path without overcommitting too early.

It also supports the intended learning curve: learn the simple tree path first, then adopt visitors, events, pull APIs, or incrementality only when the problem actually requires them.

## Relationship to Other Specs

- `framework-objectives.md` defines the requirement for strong tooling, performance, and developer ergonomics
- `parser-core-semantics.md` defines the parser-engine meaning of choice, commitment, and recovery interaction that all surfaces build on
- `parsing-profiles-and-language-modes.md` defines the profile context that all parser surfaces must respect
- `syntax-tree-and-materialization.md` defines the default CST representation and AST-lowering direction
- `recovery-and-error-boundaries.md` defines how errors and recovery actions should behave across execution models
- `source-spans-and-labels.md` defines the span and context data these models must preserve
- `tokenizer-parser-integration-guidelines.md` defines how tokenizer and parser remain one coherent pipeline

## Open Questions

1. Should the internal event model be part of the public API immediately or remain internal at first?
2. What stable node or tree identity guarantees are required before incremental parsing becomes public?
3. Should visitor support target CST, AST, or both?
4. Should top-level-item iteration be modeled as a pull parser, an iterator over parse results, or a specialized grammar facility?
5. Which of these choices should be expressed as built-in strategies versus fixed framework defaults?
6. Should incremental parsing use a distinct top-level type, or a stateful mode/session layered over `Parser`?