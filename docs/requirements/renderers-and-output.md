# Renderers and Output

## Objective

Define how diagnostics are rendered for humans and serialized for tools without changing the diagnostic model itself.

This spec covers:

- terminal rendering
- plain-text rendering
- JSON rendering
- hierarchical context-region rendering
- renderer selection and fallback behavior
- renderer configuration and width handling
- batch output versus single-diagnostic output

## Why This Needs Its Own Spec

The diagnostics model is only half the problem. Users experience diagnostics through output, and weak rendering will erase much of the value of strong spans, labels, hints, and suggestions.

This is also where library behavior must stay disciplined:

- the library must not hardcode a single output mode
- terminal behavior must remain configurable
- JSON output must preserve structure for editor, CI, and tooling integrations

## Core Principle

Renderers consume structured diagnostics. They do not own diagnostic meaning.

That means:

- spans, labels, notes, hints, and suggestions are created before rendering
- renderers may format them differently, but must not invent missing semantics
- structured information should survive losslessly in machine-readable formats

Renderer selection should be configurable, but the default path should require little or no configuration from library users.

## Likely Types

```rust
pub enum OutputFormat {
    Terminal,
    Plain,
    Json,
}

pub struct RenderOptions {
    pub use_color: bool,
    pub show_snippets: bool,
    pub width: Option<usize>,
    pub unicode: bool,
}

pub trait DiagnosticRenderer {
    fn render(&self, diagnostic: &Diagnostic) -> String;
}
```

The real implementation may split rendering from string allocation or support streaming output, but the conceptual contract should remain the same.

When multiple renderers are registered, selection should be an explicit part of the design rather than an accidental side effect of registration order.

## Requirements

1. The same diagnostic must be renderable in terminal, plain, and JSON modes.
2. Terminal and plain renderers must preserve severity, code, title, message, labels, notes, hints, and suggestions.
3. JSON output must preserve structured fields rather than flattening everything into one message string.
4. Rendering must be configurable without mutating the diagnostic itself.
5. Batch rendering must preserve diagnostic order.
6. Renderers must be able to use owning-region and ancestor-scope context when diagnostics carry it.
7. When multiple renderers are available, renderer choice must be deterministic, testable, and overridable by the caller.
8. Zero-configuration defaults must produce good human-facing output in the common case.
9. Advanced configurability must remain additive rather than mandatory.

## Configuration Philosophy

Good end-user experience means two things at once:

1. most users should not need to think about renderer selection at all
2. advanced users must still be able to override selection, output format, and fallback policy when their environment demands it

That means configurability should be layered.

Recommended layers:

1. zero-config automatic behavior for normal CLI and library use
2. simple high-value overrides such as format, color policy, width, and snippet visibility
3. advanced registration and custom renderer selection for framework integrators

Avoid requiring ordinary users to understand renderer registration, suitability scoring, or environment details just to get readable diagnostics.

## Renderer Selection Best Practices

The best practice is not a loose chain where renderers probe global state and opportunistically render.

The better model is:

1. environment detection produces a structured snapshot
2. caller or runtime builds a render request
3. a selector evaluates registered renderers against that request
4. one renderer is chosen for that output target
5. fallback happens explicitly when no higher-quality renderer matches

If a chain-of-responsibility pattern is used, it should be used only in this pure selection phase. It should not mean that each renderer mutates output or probes global state in turn.

## Why Pure Selection Is Better Than Ad Hoc Self-Nomination

Unstructured self-nomination has several problems:

- renderers may duplicate environment probing logic
- selection becomes order-sensitive in surprising ways
- behavior is harder to test deterministically
- it becomes unclear why a renderer was chosen

Industry best practice is to separate:

- environment detection
- renderer capability declaration
- renderer selection policy
- actual rendering

## Recommended Model

Likely building blocks:

```rust
pub struct RenderRequest<'a> {
    pub format_hint: Option<OutputFormat>,
    pub environment: &'a EnvironmentSnapshot,
    pub target: RenderTarget,
    pub options: &'a RenderOptions,
}

pub enum RenderTarget {
    Stdout,
    Stderr,
    Memory,
    BrowserConsole,
    Custom(&'static str),
}

pub enum RendererSuitability {
    Unsupported,
    Fallback(u8),
    Preferred(u8),
}

pub trait DiagnosticRenderer {
    fn suitability(&self, request: &RenderRequest<'_>) -> RendererSuitability;
    fn render(&self, diagnostic: &Diagnostic) -> String;
}
```

The exact API may differ, but the essential idea is:

- renderers declare suitability
- selection policy chooses the best match
- selection is data-driven and testable

## Precedence Rules

Best practice for renderer choice:

1. explicit caller-selected renderer wins
2. explicit format hint wins over auto-detection
3. target-aware preferred renderer wins over fallback renderers
4. plain-text renderer is the safe universal fallback

This avoids the worst failure mode where an eager renderer claims a context it handles poorly.

It also keeps configuration intuitive: simple explicit intent should always beat automatic policy.

## Recommended Configuration Surface

The user-facing configuration surface should stay small.

High-value knobs:

- output format: auto, terminal, plain, json
- color policy: auto, always, never
- width override
- snippet on or off
- explicit renderer override for advanced embedding scenarios

Lower-level knobs such as renderer priority, suitability scoring, or target-specific fallback chains should remain framework-level controls rather than everyday user-facing options unless real usage proves otherwise.

## Presets and Sensible Defaults

Best practice is to define a small set of behavioral presets even if the public API exposes raw configuration.

Examples:

- `Auto`: best available human-facing renderer for the current target
- `Ci`: stable plain output unless machine-readable output is explicitly requested
- `Machine`: structured JSON output
- `Debug`: human-facing output plus renderer-selection trace information

These presets improve usability because they let users express intent without understanding internal renderer policy.

## Multiple Renderers and Multiple Outputs

Multiple registered renderers does not necessarily mean one chain for one output.

There are two distinct scenarios:

1. one output target, multiple candidate renderers
2. multiple output targets, each with its own renderer

For the second case, best practice is a composite sink or fan-out runtime:

- one sink may render terminal output
- another may collect JSON diagnostics
- another may send structured output to a browser console or editor integration

Each output target should run its own renderer selection. This is cleaner than one chain trying to satisfy all outputs at once.

## Environment Detection Ownership

Renderers should not directly own environment detection by default.

Best practice:

- environment detection is centralized
- renderers consume an `EnvironmentSnapshot`
- renderers may inspect the snapshot to report suitability
- renderers should not query process-global terminal state during render selection unless explicitly configured to do so

This keeps behavior deterministic and avoids duplicated platform logic.

## Observability

The runtime should ideally be able to explain why a renderer was chosen.

Useful debug information may include:

- explicit override selected terminal renderer
- browser console renderer preferred because target was browser console
- plain renderer selected because output stream was not a TTY

This is especially useful in CI, tests, and user-reported diagnostics issues.

That observability should usually be opt-in so normal users are not exposed to renderer-selection noise.

## Terminal Rendering

Terminal output is the highest-value human-facing format.

It should support:

- colored severity and code headers when enabled
- source snippets with primary and secondary labels
- focus on the most relevant owning region when nested context exists
- readable note, hint, and suggestion blocks
- width-aware wrapping
- graceful fallback when snippets are unavailable

Terminal output should aim for parser-grade clarity rather than minimal logging output.

It should normally be selected because the target and environment support it, not because it happened to be registered first.

## Plain-Text Rendering

Plain output is for logs, CI, redirected output, and environments where ANSI is undesirable.

Requirements:

1. no ANSI escape sequences
2. still readable with multiple labels and notes
3. stable enough for snapshot-style tests when needed
4. no dependence on terminal control features

Plain output should be a deliberate renderer, not terminal rendering with colors stripped after the fact.

It should also act as the default safe fallback when richer renderers are unavailable or inappropriate.

## JSON Rendering

JSON output is the machine-readable contract.

It should preserve at least:

- schema version
- severity
- code
- title
- message
- labels and spans
- context regions and ancestor scopes when present
- notes
- hints
- suggestions

Likely top-level batch shape:

```json
{
  "schema_version": 1,
  "diagnostics": []
}
```

Single-diagnostic emission may still use the same schema by returning a one-element batch or by defining a companion single-object form. That choice should be explicit.

JSON is often the best structured fallback for integrations, but not necessarily the best human-facing fallback for interactive terminals.

## Snippet Rendering

Renderers that show source snippets depend on the source/span model.

They must handle:

- single-line spans
- multi-line spans
- insertion points
- EOF diagnostics
- missing source text
- nested context where the failure site is smaller than the most helpful display region

Snippet layout should degrade cleanly in narrow terminals rather than assuming ideal display widths.

## Scoped Context Rendering

When diagnostics include owning-region and ancestor-scope metadata, renderers should use it deliberately.

Best practices:

1. terminal output should usually show the smallest useful owning region rather than the entire ancestor chain
2. plain output should keep the same structural information in a simpler textual form
3. JSON output should preserve the full hierarchy so browsers, editors, and web consoles can render collapsible or richer context views
4. advanced renderers may show parent and ancestor regions progressively rather than all at once

The goal is to help the user understand where the failure sits inside the larger structure, not to flood the screen with full-file context.

## Suggestions Rendering

Suggestions should render differently depending on format.

- terminal: clear human guidance and replacement preview
- plain: readable text with minimal ornamentation
- JSON: structured edits and applicability metadata

The renderer should not collapse multiple alternative suggestions into one string.

## Open Questions

1. Should plain-text output have stronger stability guarantees than terminal output for snapshot testing?
2. Should JSON rendering support both compact and pretty-printed modes?
3. Should batch rendering include summary counts by severity, or should that stay outside the renderer?
4. How much of snippet layout should be configurable versus standardized?
5. Should browser-console-style structured rendering be modeled as a dedicated renderer or as a consumer of JSON output?
6. Should renderer suitability be boolean, priority-based, or score-based?