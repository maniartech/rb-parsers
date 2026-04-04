# Parsing Profiles and Language Modes

## Objective

Define how the `rb_parsers` ecosystem supports multiple language versions, strictness modes, dialect-like feature sets, and profile combinations in a way that is easy to configure and precise enough for serious language tooling.

This spec is about profile modeling and resolution, not one specific grammar DSL.

## Why This Needs Its Own Spec

Sophisticated parsers rarely target only one fixed language shape.

Real ecosystems need combinations such as:

- `v1`
- `v2`
- `v1 + strict`
- `v1 + tolerant`
- `v2 + comments`
- `v2 + legacy_compat`

If the framework does not make this first-class, language authors end up with:

- scattered booleans like `strict_mode`
- duplicate tokenizer and parser builders for each variant
- version-specific forks of entire grammars
- diagnostics that do not explain which profile rejected the syntax

That is not acceptable for a serious parsing framework.

## Core Principle

Profiles must be structured, composable, and resolved once.

That means:

- the caller describes the desired language/version/mode/feature combination
- the framework resolves that request into one validated profile
- tokenizer and parser both consume the same resolved profile
- rules and scanners are gated by profile predicates instead of scattered imperative branching

Profile compatibility must also be explicit. The framework must not guess compatibility only from profile names like `strict`, `legacy`, or `v1`.

The common path must stay easy: simple parsers should be able to use sensible defaults, while serious parsers can add more profile rules without changing the overall mental model.

## Design Goals

The system must support:

1. language versions such as `v1`, `v2`, or named editions
2. strictness or compliance modes such as `strict`, `default`, or `tolerant`
3. optional feature overlays and dialect-like extensions
4. combinations of one base profile with one or more overlays
5. deterministic conflict detection when combinations are invalid
6. easy common-case configuration for end users
7. rule-based gating so grammars do not have to be duplicated unnecessarily
8. explicit compatibility rules between profiles, overlays, and resolved profile variants
9. a small, obvious default profile path that grows naturally into advanced combinations

## Non-Goals

This system should not:

1. require users to manually assemble low-level parser internals for common profile choices
2. treat arbitrary string parsing as the primary internal representation of profile state
3. force grammar authors to duplicate entire grammars for small version or mode differences
4. mix environment-dependent runtime concerns into language-profile resolution by default

## Core Distinction: Language Profile vs Runtime Policy

The framework should separate language meaning from runtime execution policy.

Examples of language-profile concerns:

- language version
- strictness when it changes what syntax is valid
- enabled or disabled syntax features
- dialect-specific tokens or rule branches

Examples of runtime-policy concerns:

- fail-fast versus bounded recovery
- diagnostics collection or emission mode
- renderer choice
- maximum error count

These may be bundled together in higher-level presets, but they should not be the same thing internally.

## Composition vs Compatibility

The framework should distinguish clearly between:

1. composition: whether profile fragments may be combined into one request
2. compatibility: whether one resolved profile can stand in for, refine, extend, or coexist with another

These are related but not identical.

Examples:

- `v1 + strict` may be a valid composition
- `v1 + strict` may be compatible with `v1` as a stricter refinement
- `v1` may or may not be compatible with `v1 + strict`
- `v2 + legacy_compat` may be composable but still incompatible with some caching or substitution expectations

The framework should not collapse these meanings into one generic boolean.

## Likely Types

```rust
pub struct ParseProfileRequest {
    pub language: &'static str,
    pub version: Option<LanguageVersion>,
    pub mode: Option<ProfileMode>,
    pub enable_features: Vec<FeatureFlag>,
    pub disable_features: Vec<FeatureFlag>,
}

pub struct ResolvedParseProfile {
    pub id: ResolvedProfileId,
    pub language: &'static str,
    pub version: LanguageVersion,
    pub mode: ProfileMode,
    pub enabled_features: Vec<FeatureFlag>,
    pub language_profile: LanguageProfile,
}

pub struct ParseRuntimeRequest {
    pub recovery_mode: Option<RecoveryMode>,
    pub max_errors: Option<usize>,
    pub diagnostics_mode: Option<DiagnosticsMode>,
    pub renderer: Option<RendererPreference>,
}

pub struct ResolvedParseRuntime {
    pub recovery: RecoveryConfig,
    pub diagnostics: DiagnosticsConfig,
    pub rendering: RenderSelection,
}

pub struct ParseSessionConfig {
    pub profile: ResolvedParseProfile,
    pub runtime: ResolvedParseRuntime,
}

pub enum ProfileMode {
    Default,
    Strict,
    Tolerant,
    Legacy,
    Custom(&'static str),
}

pub struct RuleProfileGuard {
    pub since: Option<LanguageVersion>,
    pub until: Option<LanguageVersion>,
    pub requires_all: &'static [FeatureFlag],
    pub forbids_any: &'static [FeatureFlag],
    pub modes: &'static [ProfileMode],
}

pub enum ProfileCompatibility {
    Equivalent,
    Refines,
    Extends,
    MergeAllowed,
    Incompatible,
    Unknown,
}

pub struct ProfileCompatibilityRule {
    pub left: ProfileSelector,
    pub right: ProfileSelector,
    pub relation: ProfileCompatibility,
    pub directional: bool,
    pub reason: &'static str,
}
```

The exact types may differ, but the key requirements are:

- the profile request is ergonomic
- the resolved language profile is explicit and immutable during parsing
- stable profile identity is separated from runtime policy
- runtime policy is layered on after profile resolution
- rule gating is structured rather than ad hoc

The compatibility model must be expressive enough to support both symmetric and directional relationships.

## Resolution Model

Recommended resolution order:

1. choose a base language profile
2. apply version defaults
3. apply mode defaults
4. apply explicit feature enable or disable overlays
5. validate the resolved language profile and assign a stable normalized identity
6. resolve runtime policy defaults and caller overrides against that profile
7. build a parse session configuration before parsing starts

Compatibility evaluation should happen during this process, not after tokenizer or parser work has already started.

This makes combinations like `v1 + strict` or `v2 + comments + tolerant` easy to express without losing determinism.

## Stable Profile Identity and Parse Sessions

A resolved language profile should carry a normalized stable identifier suitable for caching, debugging, and incremental invalidation.

Requirements:

1. grammar compilation keys should depend on resolved profile identity, not renderer selection or error-budget tuning
2. runtime settings such as diagnostics sinks, renderer choice, and maximum error counts should live in runtime or session config rather than inside the language profile
3. execution caches may include additional runtime knobs only when those knobs materially affect parse behavior
4. user-facing named presets may still bundle profile and runtime defaults together, but the internal split must remain explicit

This split improves both ergonomics and performance. Callers can still ask for an editor-friendly or batch-friendly preset, while the framework keeps syntax identity, compatibility rules, and runtime policy disentangled.

## Compatibility Model

Profile compatibility should be explicit and preferably directional.

Recommended interpretations:

1. `Equivalent`: both profiles are meaningfully interchangeable
2. `Refines`: the left profile is a stricter or narrower form of the right profile
3. `Extends`: the left profile adds capabilities beyond the right profile
4. `MergeAllowed`: the two profile fragments may be composed into one resolved profile, but that does not imply interchangeability
5. `Incompatible`: the profiles must not be combined or substituted
6. `Unknown`: no rule has established a safe relationship

Why direction matters:

- `v1 + strict` may refine `v1`
- `v1` does not automatically refine `v1 + strict`

That relationship is not symmetric, so the model must not force compatibility to mean only `true` or `false`.

## Safe Defaults

The framework should be conservative by default.

Recommended default behavior:

1. exact identity is compatible with itself
2. all non-identical relationships are `Unknown` unless declared by rules
3. `Unknown` should be treated as not safely substitutable
4. composition should fail unless the requested combination is explicitly allowed by base-profile or overlay rules

This avoids dangerous assumptions such as treating every `strict` profile as automatically compatible with its non-strict base.

## Declaring Compatibility Rules

Language authors should be able to declare compatibility rules in a structured way.

Examples of useful declarations:

- `v1 + strict` refines `v1`
- `v2 + comments` extends `v2`
- `legacy_compat` is merge-allowed with `v2`
- `strict` is incompatible with `legacy_compat`

Best practice is to attach these rules to profile metadata or a profile catalog rather than scattering them through parser logic.

## Compatibility Use Cases

The compatibility model should support at least these decisions:

1. whether two profile fragments may be combined during resolution
2. whether one resolved profile may reuse cached grammar state from another profile
3. whether one parser configuration may safely satisfy a request for another profile
4. whether diagnostics should suggest a nearby compatible profile when the requested combination is invalid
5. whether a feature overlay narrows, extends, or conflicts with a base profile

These are the places where a strong rule model pays off.

## Examples

Example directional refinement:

- `json/v1+strict` refines `json/v1`
- `json/v1` is not automatically compatible with `json/v1+strict`

Example explicit incompatibility:

- `expr/v2+legacy_compat` incompatible with `expr/v2+strict`

Example merge-only relationship:

- `v2` merge-allowed with `comments`
- `v2 + comments` is not necessarily equivalent to plain `v2`

## Compatibility and Diagnostics

When compatibility rules reject a request, diagnostics should say why.

Good examples:

- "The `strict` mode refines `v1`, but it is incompatible with the `legacy_compat` feature."
- "The `comments` overlay can be merged with `v2`, but it is not available for `v1`."

When useful, diagnostics may also mention a nearby compatible profile or valid combination.

## Named Presets and Structured Builders

The framework should support both:

1. a simple named or shorthand preset for common cases
2. a structured builder for advanced and tooling-driven cases

This is not optional polish. It is part of the design goal that the easy path and the serious path use the same conceptual system.

Examples of acceptable user-facing shapes:

```rust
let profile = profiles.resolve_named("json/v1+strict")?;
```

```rust
let profile = profiles
    .request("json")
    .version("v1")
    .mode(ProfileMode::Strict)
    .enable("comments")
    .resolve()?;
```

Best practice is to keep the string form as convenience only. The internal representation must remain structured.

## Easy Path Versus Advanced Path

Best practice is a layered profile API.

Recommended layers:

1. implicit default profile for the most common parser configuration
2. named preset selection for common variants such as `v1`, `v1+strict`, or `v2`
3. structured request building for advanced combinations and tooling integrations
4. explicit compatibility and composition rules for serious language evolution scenarios

This keeps the first experience small while still allowing language authors to grow into more complex profile systems.

Language authors should not have to choose between:

- a simple system that cannot evolve
- a powerful system that is painful to learn

The framework should provide both by making the power rule-based and layered.

## Rule-Based Gating

The framework should let grammar authors express profile-dependent behavior declaratively.

That means scanners, token definitions, parser rules, and recovery strategies may be guarded by profile predicates such as:

- only valid since version `v2`
- forbidden in strict mode
- enabled only when `comments` feature is active
- disabled when `legacy_compat` is enabled

Best practice is to express these differences as rule guards or feature predicates, not as scattered `if profile.strict_mode { ... }` branches deep inside parse logic.

This is also what keeps the system easy to implement and learn: one predictable rule model, not many special cases.

## Build-Time and Parse-Time Behavior

If possible, the framework should resolve profile-dependent rule availability before hot parsing begins.

Preferred strategies:

1. precompute enabled rule and scanner sets from the resolved profile
2. compile or cache profile-specialized grammar views when beneficial
3. avoid repeated dynamic profile checks on every token transition when the active profile is already known

This matters because a rich profile system must not destroy parser performance.

## Tokenizer and Parser Coordination

Profiles are pipeline-wide.

Requirements:

1. tokenizer and parser must consume the same `ResolvedParseProfile`
2. tokenizer should only vary by profile when lexical syntax really changes
3. parser should own profile-gated grammar behavior that is syntactic rather than lexical
4. diagnostics must be able to mention the relevant profile or feature when it explains the failure

This avoids the class of bugs where tokenization runs in `v1` mode while parsing assumes `v2`.

## Diagnostics Expectations

Profile-aware parsing should produce profile-aware diagnostics.

Examples:

- syntax only valid in `v2`
- construct forbidden in strict mode
- extension feature required but not enabled
- requested profile combination is internally inconsistent

These diagnostics should be specific.

Good guidance:

- "Trailing commas are only allowed in v2 or when the `trailing_commas` feature is enabled."
- "This legacy escape form is disabled in strict mode."

Bad guidance:

- "Invalid syntax."

## Conflict Detection

Invalid combinations should fail during profile resolution rather than producing confusing downstream parse behavior.

Examples of conflicts:

- enabling mutually exclusive features
- requesting `strict` mode with a feature marked `strict_incompatible`
- requesting a feature introduced after the selected version
- requesting two profiles whose compatibility rules resolve to `Incompatible`
- requesting substitution or cache reuse where compatibility is only `Unknown`

Resolution errors should be structured and diagnostic-friendly.

## Common Profile Shapes

The design should support at least these patterns:

1. one version with strict and tolerant variants
2. multiple versions with shared core grammar and a few guarded differences
3. one base language plus optional extensions
4. compatibility or legacy modes that relax or reinterpret a limited set of rules

The design should not force every language into the same profile taxonomy, but it should give all of them a common framework.

## Testing Guidance

Profile support should be tested with discipline.

Recommended coverage:

1. resolution tests for valid and invalid profile combinations
2. tokenizer and parser agreement tests for the same resolved profile
3. regression tests for constructs that differ by version
4. strict versus tolerant behavior tests
5. feature-guard tests that prove enabled and disabled branches behave correctly
6. diagnostics tests that assert profile-specific error codes or messages where important
7. compatibility tests for directional refinement, equivalence, merge-only, and incompatibility rules

Avoid exploding the test matrix blindly. Prefer representative combinations plus focused tests for each guarded feature or mode boundary.

## Recommended API Direction

The common-case API should feel small.

Examples:

```rust
let profile = profiles.default_for("expr")?;
let tokens = tokenizer.tokenize_with_profile(input, &profile)?;
let tree = parser.parse_with_profile(&tokens, &profile)?;
```

```rust
let profile = profiles.resolve_named("expr/v2+strict")?;
let tokens = tokenizer.tokenize_with_profile(input, &profile)?;
let tree = parser.parse_with_profile(&tokens, &profile)?;
```

Advanced embedding should still be possible:

```rust
let request = ParseProfileRequest {
    language: "expr",
    version: Some(LanguageVersion::new("v2")),
    mode: Some(ProfileMode::Strict),
    enable_features: vec![FeatureFlag::new("comments")],
    disable_features: vec![],
};

let profile = profile_catalog.resolve(request)?;
let runtime = runtime_catalog
    .request()
    .recovery_mode(RecoveryMode::ContinueBounded)
    .max_errors(20)
    .resolve_for(&profile)?;

let session = ParseSessionConfig { profile, runtime };
```

The easy path should look boring in a good way. The advanced path should feel like a direct extension of the same model, not a different framework.

## Relationship to Other Specs

- `framework-objectives.md` defines the product expectation that multi-profile parsing should be first-class
- `tokenizer-parser-integration-guidelines.md` defines how one resolved profile flows through the pipeline
- `recovery-and-error-boundaries.md` defines recovery behavior that may be influenced by profile-specific policy presets
- `error-system.md` defines how profile-related validation and parse failures are diagnosed

## Open Questions

1. Should profile predicates be expressed directly in the grammar DSL, or attached through metadata APIs?
2. What public stable identifier type should resolved profiles expose for caching, debugging, and incremental invalidation?
3. How much of profile specialization should happen eagerly versus lazily?
4. Should strictness always be modeled as `ProfileMode`, or can languages define richer custom mode taxonomies?
5. Should the framework provide a standard preset parser for strings like `language/version+feature+strict`, or leave that to language crates?
6. Should compatibility rules be declared as a matrix, as profile metadata, or as executable predicates in the profile catalog?
7. Which runtime-policy knobs, if any, are allowed to influence incremental reuse keys?