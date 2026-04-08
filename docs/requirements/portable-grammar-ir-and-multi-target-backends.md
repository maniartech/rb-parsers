# Portable Grammar IR and Multi-Target Backends

## Objective

Define how `rb_parsers` can eventually let one language definition drive Rust and non-Rust parser backends without forcing grammar authors to rewrite the same DSL for each host language.

This spec is about backend-neutral grammar representation and portability boundaries, not evaluator semantics or one specific code generator.

## Why This Needs Its Own Spec

For many real projects, especially mini DSLs embedded in multiple applications, parser performance is only part of the value.

Another genuine requirement is minimizing developer effort.

If language authors must:

- define the grammar in Rust for one project
- reimplement it in TypeScript for another
- reimplement it again in Python, Go, or another host

then the framework has failed an important product goal even if each individual runtime is fast.

The project therefore needs a clear answer to this question:

- what part of the language definition is portable
- what part stays backend-specific
- how Rust-first design choices avoid blocking future target backends

## Core Principle

One canonical language definition should be able to feed multiple backends.

That means:

- grammar authoring should happen once in a normalized, backend-neutral form
- Rust should be the first reference backend, not the only possible backend forever
- future non-Rust targets should consume the same portable grammar IR rather than a second grammar DSL
- backend-specific escape hatches should exist, but remain explicit and clearly non-portable

The portability goal is not “generate everything everywhere immediately.”

The goal is to reduce duplicated parser authoring effort while preserving clear architecture and competitive performance.

## Product Requirement: Minimize Repeated Porting Work

Developer-effort minimization is not optional polish.

It is a real design requirement.

That means the framework should help users avoid this failure mode:

1. define a DSL once in Rust
2. discover the same DSL is needed in another host language
3. start searching for a separate parser generator or ad hoc porting mechanism
4. maintain multiple grammar implementations that drift over time

The framework should instead create a future path where one language definition can remain the source of truth.

## Portable Core Versus Backend-Specific Layers

The architecture should distinguish clearly between:

1. portable grammar meaning
2. backend-specific runtime realization
3. language-specific semantic or evaluator layers

Portable grammar meaning should include:

- token kinds and token identities
- syntax kinds
- rule graph and structure
- precedence and associativity
- profile guards and feature gating
- recovery landmarks and synchronization boundaries
- diagnostics metadata that can survive across backends

Backend-specific realization may include:

- runtime data structures
- memory layout
- parser execution optimization strategy
- emitted source format and packaging
- renderer integrations for that host environment

Semantic and evaluator layers should remain outside the initial portable grammar core because they are usually much more language-specific and host-specific than parsing.

## Backend-Neutral Grammar IR

The project should eventually define a normalized grammar IR that is:

- deterministic
- serializable or at least exportable in a stable form
- validated before backend emission
- expressive enough for CST-first parsing, profile guards, and recovery landmarks
- restrictive enough that portability is real rather than mostly aspirational

Conceptually:

```rust
pub struct PortableGrammarIr {
    pub tokens: TokenSetIr,
    pub rules: Vec<RuleIr>,
    pub syntax_kinds: Vec<SyntaxKindIr>,
    pub profiles: ProfileIr,
    pub recovery: RecoveryIr,
}
```

The exact shape may differ, but the architectural role matters more than the exact fields.

## Rust as the First Reference Backend

Rust should remain the first-class backend while the architecture is proven.

Recommended stance:

1. the Rust runtime is the reference implementation for semantics and performance
2. the normalized IR should first compile cleanly into the Rust parser runtime
3. future target backends should be judged against the same normalized semantics rather than inventing their own interpretations

This keeps the project grounded while preserving a future path to multi-target support.

## Preferred Portability Strategy

When the goal is to use the same parser in multiple host languages, the first preference should usually be to reuse the Rust implementation rather than immediately generating native parser code for every target.

Preferred order when applicable:

1. direct Rust backend for Rust consumers
2. Rust backend exposed through a C-compatible ABI for native embedding
3. Rust backend packaged through WebAssembly for browser or embedding scenarios
4. Rust backend packaged through WASI when a component-style or sandboxed runtime is the right deployment boundary
5. emitted native source backends only when host-language integration requirements make the Rust-backed options insufficient

Why this should be the default:

- it preserves one high-performance reference implementation
- it minimizes semantic drift between targets
- it reduces the number of runtimes that must independently implement parser-core semantics, recovery, and profile behavior
- it lowers total maintenance cost for mini DSLs that need broad host reach more than host-native implementation purity

This means multi-host support and native code generation should not be treated as synonyms.

## Emission Targets

The long-term model may support more than one backend shape.

Examples:

1. compiled Rust backend for direct in-process use
2. Rust backend exposed through C bindings for native consumers in other host languages
3. Rust backend compiled to WebAssembly or WASI for embeddable or sandboxed execution
4. emitted source backend such as TypeScript or Python when a host-native runtime is required
5. generated tables plus a small host-language runtime
6. serialized IR consumed by another build step or generator

The important design rule is that these are all backend realizations of the same language definition, not separate language-definition systems.

The early portability story should prefer items 2 and 3 over item 4 whenever that meets product needs.

## Host-Specific Escape Hatches

Some features will inevitably be backend-specific.

Examples:

- scanner hooks that depend on a host regex engine
- runtime callbacks tied to one host language type system
- target-specific memory or allocation strategies

These should be allowed only as explicit escape hatches.

Requirements:

1. non-portable features should be labeled clearly
2. portable backends should be able to reject unsupported host-specific features during validation
3. the common path should keep grammar authors inside the portable subset by default

## Semantic Actions Are the Main Portability Risk

The canonical grammar layer should not depend on host-language closures or arbitrary host-language code to express syntax meaning.

That means:

- structure-first grammar should remain the default
- AST lowering should be layered after parsing
- host-language-specific semantic actions should not be required for ordinary grammar authoring
- if semantic hooks exist, they should be clearly marked as backend-specific and non-portable

This is one of the strongest reasons to keep the grammar model CST-first and structure-first.

## Tokenization Portability

Tokenizer portability deserves explicit discipline.

Recommended direction:

1. define a portable lexical subset for symbols, common regex classes, and scanner ordering
2. allow host-specific scanner extensions only as explicit non-portable features
3. prefer portable token identity and ordering rules over backend-specific lexical cleverness in the canonical grammar layer

Different regex engines and text models will diverge. The framework should acknowledge that up front instead of pretending every lexical feature is portable automatically.

## Diagnostics and Recovery Portability

Cross-language backends should aim for semantic parity before exact presentation parity.

That means:

- same accepted or rejected syntax meaning
- same high-level error codes where possible
- same profile-guard behavior
- same recovery boundary semantics where practical

It does not require:

- byte-for-byte identical renderer output across every target
- identical snippet layout in every host environment from day one

This keeps the portability goal realistic.

## Capability Checking

Backends should be able to declare what they support.

Conceptually:

```rust
pub struct BackendCapabilities {
    pub profile_guards: bool,
    pub bounded_recovery: bool,
    pub event_surface: bool,
    pub portable_regex_subset_only: bool,
}
```

This allows the framework to reject or warn about grammar features a target backend cannot yet honor safely.

## Developer Experience Expectations

The portability direction should reduce user effort, not multiply configuration burden.

Requirements:

1. grammar authors should not need to learn a second grammar DSL just to target another host language
2. backend-neutral authoring should be the default path
3. backend selection should feel like choosing a deployment target, not rewriting the grammar
4. portability restrictions should produce clear diagnostics rather than hidden degradation

For many users, this should mean choosing between:

- Rust library output
- C ABI package
- WebAssembly package
- WASI package
- host-native emitted source only when necessary

The product goal is one language definition, many realizations, not one framework per target language.

## Recommended Phasing

Reasonable implementation order:

1. stabilize the structure-first grammar model in Rust
2. define a normalized grammar IR and portable subset boundaries
3. prove at least one Rust-backed cross-host portability surface such as C ABI or WebAssembly/WASI packaging
4. prove one non-Rust native emitted backend only if product needs remain unmet by the Rust-backed portability surfaces
5. expand backend capabilities carefully based on proven demand
6. consider evaluator or semantic frameworks only after parsing portability is established

This keeps the project ambitious without turning it into an all-backends-at-once rewrite.

## Relationship to Other Specs

- `framework-objectives.md` defines minimizing developer effort and repeated porting work as an important product concern
- `parser-core-semantics.md` defines the semantics that all backends must preserve
- `parser-execution-and-consumption-models.md` defines the parser surfaces that a backend may expose or emulate
- `syntax-tree-and-materialization.md` defines the CST-first structure that motivates a portable structure-first grammar model
- `parsing-profiles-and-language-modes.md` defines portable profile resolution and guarded language variation
- `tokenizer-parser-integration-guidelines.md` defines pipeline responsibilities that backends should preserve

## Open Questions

1. What should the first non-Rust target backend be?
2. How strict should the initial portable lexical subset be?
3. Should the normalized grammar IR be a public stable artifact or initially an internal compiler boundary?
4. Which backend-specific escape hatches are important enough to support early without undermining portability?
5. At what point does evaluator or semantic portability become worth its own spec layer rather than remaining intentionally out of scope?
6. Which packaging surface should be proven first: C ABI, WebAssembly, or WASI?