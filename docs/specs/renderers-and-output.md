# Spec: Renderers and Output

**Status**: Ready for implementation
**Module**: `rb_common::render`
**Depends on**: `rb_common::spans`, `rb_common::diagnostics`, `rb_common::env`
**Requirement source**: `docs/requirements/renderers-and-output.md`

---

## Decisions Made

| Question | Decision |
|---|---|
| Plain-text snapshot stability | Yes. Plain output has a stable format contract; terminal output may change presentation. Tests should use plain output for snapshot assertions. |
| JSON compact vs pretty | Controlled by `RenderOptions::json_pretty: bool`. Default is compact. |
| Batch summary counts | Outside the renderer; callers inspect `DiagnosticsContext::error_count()` |
| Snippet layout configurability | Standardized with two knobs: `context_lines` and `max_width` |
| Browser console renderer | A consumer of JSON output, not a separate renderer type |
| Renderer suitability | Score-based (`u8`). `Preferred(score)` > `Fallback(score)` > `Unsupported`. Stable sort by variant then score. |
| Source text access | Renderers receive an optional `&dyn SourceStore` to extract snippet lines. Renderers that cannot receive source text still produce output; they omit snippet sections gracefully. |
| `render_to_string` vs `render_diagnostics` | Both exist. `render_to_string` renders one `Diagnostic`. `render_diagnostics` renders all diagnostics from a `DiagnosticsContext` using a `RenderOutputPreset`. The parser-developer-workflow and README use `render_diagnostics`. |

---

## Module Layout

```
rb_common::render
├── SourceStore          (trait — source text access bridge)
├── SnippetLines         (extracted snippet data)
├── SourceLine           (one line in a snippet)
├── OutputFormat
├── RenderTarget
├── RenderOptions
├── RenderOutputPreset
├── RenderRequest
├── RendererSuitability
├── DiagnosticRenderer  (trait)
├── RendererSelector
├── TerminalRenderer
├── PlainRenderer
├── JsonRenderer
├── render_to_string()  (render one Diagnostic)
└── render_diagnostics() (render all from DiagnosticsContext)
```

---

## Types

### SourceStore (trait)

Bridge that allows renderers to fetch source text for snippet rendering.
Callers provide this when they have the source available; it is always
optional so renderers degrade gracefully when source is absent.

```rust
use crate::spans::SourceId;

/// Access to original source text, keyed by `SourceId`.
/// Implement this on whatever type owns the source buffers in your application.
pub trait SourceStore: Send + Sync {
    /// Returns the full source text for the given source, or `None` if the
    /// source is not available in this store.
    fn source_text(&self, source_id: SourceId) -> Option<&str>;

    /// Returns a display name (e.g. file path) for the given source, or `None`.
    fn display_name(&self, source_id: SourceId) -> Option<&str>;
}
```

---

### SnippetLines

The extracted snippet data produced from a `SourceStore` + `SourceSpan`.
Renderers use this to format the `-->` and `|` lines in plain and terminal
output rather than formatting raw source text directly.

```rust
/// A rendered view of the source lines around a diagnostic span.
pub struct SnippetLines {
    /// Display name of the source file (may be an empty string).
    pub source_name: String,
    /// Lines extracted from the source, including context lines.
    pub lines: Vec<SourceLine>,
    /// The span that was requested. Used to compute column indicators.
    pub primary_byte_range: std::ops::Range<usize>,
}

/// One source line in a snippet.
pub struct SourceLine {
    /// 1-based line number.
    pub line_number: usize,
    /// The text content of this line (no trailing newline).
    pub content: String,
    /// `true` for lines that are included for context only (not part of the
    /// primary span); `false` if this line contains part of the primary span.
    pub is_context: bool,
}

impl SnippetLines {
    /// Extract snippet lines from a `SourceStore` for the given span, with
    /// `context_lines` lines of surrounding context above and below.
    pub fn extract(
        store: &dyn SourceStore,
        source_id: SourceId,
        primary_byte_range: std::ops::Range<usize>,
        context_lines: usize,
    ) -> Option<Self> {
        let text = store.source_text(source_id)?;
        let source_name = store.display_name(source_id)
            .unwrap_or("<unknown>")
            .to_string();
        // Collect (0-based byte offset of line start, line content)
        let line_starts: Vec<usize> = std::iter::once(0)
            .chain(text.char_indices()
                .filter(|(_, c)| *c == '\n')
                .map(|(i, _)| i + 1))
            .collect();
        // Find which lines contain the primary span
        let first_line = line_starts.partition_point(|&s| s <= primary_byte_range.start).saturating_sub(1);
        let last_line  = line_starts.partition_point(|&s| s < primary_byte_range.end).saturating_sub(1);
        let start_line = first_line.saturating_sub(context_lines);
        let end_line   = (last_line + context_lines).min(line_starts.len().saturating_sub(1));
        let mut lines = Vec::new();
        for i in start_line..=end_line {
            let line_start = line_starts[i];
            let line_end = if i + 1 < line_starts.len() {
                line_starts[i + 1].saturating_sub(1) // exclude '\n'
            } else {
                text.len()
            };
            lines.push(SourceLine {
                line_number: i + 1,
                content: text[line_start..line_end].to_string(),
                is_context: i < first_line || i > last_line,
            });
        }
        Some(SnippetLines { source_name, lines, primary_byte_range })
    }
}
```

---

### OutputFormat

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    /// Choose the best human-facing format based on environment.
    #[default]
    Auto,
    /// ANSI-colored terminal output with snippets.
    Terminal,
    /// Plain text without ANSI sequences; stable for logs and CI.
    Plain,
    /// Structured JSON following the schema defined below.
    Json,
}
```

---

### RenderTarget

Identifies where the output is going so renderers can score their suitability.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RenderTarget {
    #[default]
    Stdout,
    Stderr,
    /// In-memory string; the caller will use it directly.
    Memory,
    /// Custom label for advanced embedding scenarios.
    Custom(&'static str),
}
```

---

### RenderOptions

All renderer configuration in one place.

```rust
use crate::env::ColorPreference;

#[derive(Debug, Clone)]
pub struct RenderOptions {
    /// Explicit format override. `Auto` resolves at render time.
    pub format: OutputFormat,
    /// Color policy override for the target stream.
    pub color: ColorPreference,
    /// Width in columns. `None` → defer to environment snapshot or use 80.
    pub width: Option<usize>,
    /// Whether to include source snippets in terminal and plain output.
    pub show_snippets: bool,
    /// Whether to emit unicode box-drawing characters in terminal output.
    pub unicode: bool,
    /// Number of context lines shown above and below the primary span.
    pub context_lines: usize,
    /// Whether to pretty-print JSON output (indent: 2 spaces).
    pub json_pretty: bool,
}

impl Default for RenderOptions {
    fn default() -> Self {
        RenderOptions {
            format: OutputFormat::Auto,
            color: ColorPreference::Auto,
            width: None,
            show_snippets: true,
            unicode: true,
            context_lines: 1,
            json_pretty: false,
        }
    }
}

impl RenderOptions {
    pub fn plain() -> Self {
        RenderOptions {
            format: OutputFormat::Plain,
            color: ColorPreference::Never,
            ..Default::default()
        }
    }

    pub fn json() -> Self {
        RenderOptions {
            format: OutputFormat::Json,
            ..Default::default()
        }
    }

    pub fn json_pretty() -> Self {
        RenderOptions {
            format: OutputFormat::Json,
            json_pretty: true,
            ..Default::default()
        }
    }
}
```

---

### RenderOutputPreset

Named behavioral presets that cover the most common output scenarios.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderOutputPreset {
    /// Best available human output for the current terminal/environment.
    Auto,
    /// Stable plain output for CI pipelines without color dependencies.
    Ci,
    /// Structured JSON for machine consumers.
    Machine,
    /// Human output with renderer-selection trace for debugging.
    Debug,
}

impl RenderOutputPreset {
    pub fn to_options(self) -> RenderOptions {
        match self {
            RenderOutputPreset::Auto => RenderOptions::default(),
            RenderOutputPreset::Ci => RenderOptions {
                format: OutputFormat::Plain,
                color: ColorPreference::Never,
                unicode: false,
                ..Default::default()
            },
            RenderOutputPreset::Machine => RenderOptions::json(),
            RenderOutputPreset::Debug => RenderOptions {
                format: OutputFormat::Terminal,
                show_snippets: true,
                context_lines: 3,
                ..Default::default()
            },
        }
    }
}
```

---

### RenderRequest

The immutable context passed to renderer suitability scoring.

```rust
use crate::env::EnvironmentSnapshot;

pub struct RenderRequest<'a> {
    pub format_hint: Option<OutputFormat>,
    pub environment: &'a EnvironmentSnapshot,
    pub target: RenderTarget,
    pub options: &'a RenderOptions,
}
```

---

### RendererSuitability

Score-based selection. `Preferred(score)` is always preferred over any
`Fallback(score)`.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RendererSuitability {
    /// This renderer cannot handle the request at all.
    Unsupported,
    /// This renderer can produce acceptable output but is not the best choice.
    Fallback(u8),
    /// This renderer is the right choice for this request.
    Preferred(u8),
}
```

---

### DiagnosticRenderer (trait)

```rust
use crate::diagnostics::Diagnostic;

pub trait DiagnosticRenderer: Send + Sync {
    /// Score this renderer's fitness for the given request.
    fn suitability(&self, request: &RenderRequest<'_>) -> RendererSuitability;

    /// Render one diagnostic to a `String`. Must not fail.
    /// `source` is optional; when present the renderer may include source
    /// snippets. When absent snippet sections are omitted silently.
    fn render(
        &self,
        diagnostic: &Diagnostic,
        options: &RenderOptions,
        source: Option<&dyn SourceStore>,
    ) -> String;

    /// Optional: render an ordered batch of diagnostics.
    /// Default implementation renders them individually and joins with newlines.
    fn render_batch(
        &self,
        diagnostics: &[Diagnostic],
        options: &RenderOptions,
        source: Option<&dyn SourceStore>,
    ) -> String {
        diagnostics
            .iter()
            .map(|d| self.render(d, options, source))
            .collect::<Vec<_>>()
            .join("\n")
    }
}
```

---

### RendererSelector

Pure, data-driven selection. Chooses the highest-scoring renderer for a
given `RenderRequest`.

```rust
pub struct RendererSelector {
    renderers: Vec<Box<dyn DiagnosticRenderer>>,
}

impl RendererSelector {
    pub fn new() -> Self {
        RendererSelector { renderers: Vec::new() }
    }

    /// Defaults: terminal, plain (fallback), json.
    pub fn default_renderers() -> Self {
        let mut s = RendererSelector::new();
        s.renderers.push(Box::new(TerminalRenderer::default()));
        s.renderers.push(Box::new(PlainRenderer::default()));
        s.renderers.push(Box::new(JsonRenderer::default()));
        s
    }

    pub fn add(mut self, renderer: Box<dyn DiagnosticRenderer>) -> Self {
        self.renderers.push(renderer);
        self
    }

    /// Returns the best-fit renderer for `request`, or `None` when all
    /// renderers report `Unsupported`.
    pub fn select<'a>(&'a self, request: &RenderRequest<'_>) -> Option<&'a dyn DiagnosticRenderer> {
        self.renderers
            .iter()
            .filter_map(|r| {
                let s = r.suitability(request);
                if s == RendererSuitability::Unsupported {
                    None
                } else {
                    Some((s, r.as_ref()))
                }
            })
            .max_by_key(|(s, _)| *s)
            .map(|(_, r)| r)
    }
}
```

---

### TerminalRenderer

ANSI-colored terminal output with snippets.

```rust
#[derive(Debug, Default)]
pub struct TerminalRenderer;

impl DiagnosticRenderer for TerminalRenderer {
    fn suitability(&self, request: &RenderRequest<'_>) -> RendererSuitability {
        let env = request.environment;
        let is_tty = match request.target {
            RenderTarget::Stdout => env.stdout_is_tty,
            RenderTarget::Stderr => env.stderr_is_tty,
            _ => false,
        };
        let explicit_terminal = matches!(
            request.format_hint.or(Some(request.options.format)),
            Some(OutputFormat::Terminal)
        );
        if explicit_terminal {
            return RendererSuitability::Preferred(200);
        }
        if is_tty && env.effective_color(is_tty) {
            RendererSuitability::Preferred(100)
        } else {
            RendererSuitability::Unsupported
        }
    }

    fn render(
        &self,
        diagnostic: &Diagnostic,
        options: &RenderOptions,
        _source: Option<&dyn SourceStore>,
    ) -> String {
        // Implementation produces ANSI-colored output.
        // Full implementation is in rb_common::render::terminal (internal module).
        todo!("terminal renderer implementation")
    }
}
```

---

### PlainRenderer

No ANSI sequences. Stable format for logs, CI, and snapshot tests.

```rust
#[derive(Debug, Default)]
pub struct PlainRenderer;

impl DiagnosticRenderer for PlainRenderer {
    fn suitability(&self, request: &RenderRequest<'_>) -> RendererSuitability {
        match request.format_hint.or(Some(request.options.format)) {
            Some(OutputFormat::Plain) => RendererSuitability::Preferred(200),
            Some(OutputFormat::Json) => RendererSuitability::Unsupported,
            // Plain is the universal fallback for non-terminal, non-JSON targets.
            _ => RendererSuitability::Fallback(50),
        }
    }

    fn render(
        &self,
        diagnostic: &Diagnostic,
        _options: &RenderOptions,
        _source: Option<&dyn SourceStore>,
    ) -> String {
        // Produces exactly:
        //   {severity}[{code}]: {message}
        //     --> {file}:{line}:{column}
        //      | {snippet line}
        //      | {label}
        //      = hint: {hint}
        //      = note: {note}
        //   suggestion: {title} ({applicability})
        todo!("plain renderer implementation")
    }
}
```

---

### JsonRenderer

Machine-readable output. Schema version is stable once 1.0 is released.

```rust
#[derive(Debug, Default)]
pub struct JsonRenderer;

impl DiagnosticRenderer for JsonRenderer {
    fn suitability(&self, request: &RenderRequest<'_>) -> RendererSuitability {
        match request.format_hint.or(Some(request.options.format)) {
            Some(OutputFormat::Json) => RendererSuitability::Preferred(200),
            _ => RendererSuitability::Unsupported,
        }
    }

    fn render(
        &self,
        diagnostic: &Diagnostic,
        options: &RenderOptions,
        _source: Option<&dyn SourceStore>,
    ) -> String {
        // Schema_version: 1
        // Single-diagnostic form wraps in { "schema_version": 1, "diagnostics": [ ... ] }
        todo!("json renderer implementation")
    }
}
```

#### JSON Schema (version 1) — minimal example

```json
{
  "schema_version": 1,
  "diagnostics": [
    {
      "severity": "error",
      "code": "RBT-unrecognized-char",
      "title": "Unrecognized character",
      "message": "unrecognized character `@` at this position",
      "labels": [
        {
          "style": "primary",
          "location": {
            "kind": "real",
            "source_id": 1,
            "start": { "byte_offset": 42, "line": 2, "column": 10 },
            "end":   { "byte_offset": 43, "line": 2, "column": 11 }
          },
          "message": null
        }
      ],
      "context": null,
      "notes": [],
      "hints": [
        { "text": "If this character is valid, add a scanner for it.", "is_auto": false }
      ],
      "suggestions": []
    }
  ]
}
```

#### JSON Schema (version 1) — fully populated example

This shows all optional fields populated to make the schema unambiguous for
implementors and consumer tooling.

```json
{
  "schema_version": 1,
  "diagnostics": [
    {
      "severity": "error",
      "code": "RBP-missing-token",
      "title": "Missing token",
      "message": "expected `:` after member key but found `}`",
      "labels": [
        {
          "style": "primary",
          "location": {
            "kind": "real",
            "source_id": 1,
            "start": { "byte_offset": 88, "line": 5, "column": 3 },
            "end":   { "byte_offset": 89, "line": 5, "column": 4 }
          },
          "message": "`:` expected here"
        },
        {
          "style": "secondary",
          "location": {
            "kind": "real",
            "source_id": 1,
            "start": { "byte_offset": 80, "line": 5, "column": 1 },
            "end":   { "byte_offset": 87, "line": 5, "column": 8 }
          },
          "message": "member key opened here"
        }
      ],
      "context": {
        "kind": "enclosing_node",
        "name": "JsonObject",
        "location": {
          "kind": "real",
          "source_id": 1,
          "start": { "byte_offset": 0, "line": 1, "column": 1 },
          "end":   { "byte_offset": 120, "line": 8, "column": 1 }
        }
      },
      "notes": [
        "JSON object members must be key-colon-value triples."
      ],
      "hints": [
        { "text": "Add a `:` between the key and the value.", "is_auto": false },
        { "text": "If you intended a bare identifier, wrap it in double quotes.", "is_auto": true }
      ],
      "suggestions": [
        {
          "title": "Insert missing `:`",
          "applicability": "MachineApplicable",
          "edits": [
            {
              "source_id": 1,
              "start": { "byte_offset": 88, "line": 5, "column": 3 },
              "end":   { "byte_offset": 88, "line": 5, "column": 3 },
              "replacement": ":"
            }
          ]
        }
      ]
    }
  ]
}
```

**Schema notes:**

- `context` — present when diagnostics carry an enclosing-scope region. The
  `kind` field is one of `"enclosing_node"`, `"enclosing_scope"`, or
  `"ancestor_chain"`.
- `suggestions[].applicability` — one of `"MachineApplicable"`, `"MaybeIncorrect"`,
  `"HasPlaceholders"`, `"Unspecified"`.
- `suggestions[].edits` — ordered array of non-overlapping text edits. Each edit
  is a half-open `[start, end)` byte range that is replaced by `replacement`.
- `hints[].is_auto` — `true` if generated by the framework; `false` if authored
  by the grammar/language definition.

---

### render_to_string (free function)

Renders a single `Diagnostic` with automatic renderer selection.

```rust
/// Selects the best renderer for the given environment and renders `diagnostic`.
/// Falls back to `PlainRenderer` when no renderer matches.
/// `source` is optional; when present, renderers include source snippets.
pub fn render_to_string(
    diagnostic: &Diagnostic,
    options: &RenderOptions,
    env: &EnvironmentSnapshot,
    target: RenderTarget,
    source: Option<&dyn SourceStore>,
) -> String {
    let request = RenderRequest {
        format_hint: None,
        environment: env,
        target,
        options,
    };
    let selector = RendererSelector::default_renderers();
    match selector.select(&request) {
        Some(renderer) => renderer.render(diagnostic, options, source),
        None => PlainRenderer::default().render(diagnostic, options, source),
    }
}
```

---

### render_diagnostics (free function)

The primary call site used in parser-developer-workflow and README examples.
Renders all diagnostics from a `DiagnosticsContext` using a named preset.

```rust
use crate::diagnostics::DiagnosticsContext;

/// Render all diagnostics collected in `ctx` using the best renderer for
/// `preset`. Diagnostics are rendered in emission order.
/// `source` is optional; when present, renderers include source snippets.
pub fn render_diagnostics(
    ctx: &DiagnosticsContext,
    preset: RenderOutputPreset,
    source: Option<&dyn SourceStore>,
) -> String {
    let options = preset.to_options();
    let env = EnvironmentSnapshot::detect();
    let request = RenderRequest {
        format_hint: None,
        environment: &env,
        target: RenderTarget::Memory,
        options: &options,
    };
    let selector = RendererSelector::default_renderers();
    let renderer = selector
        .select(&request)
        .unwrap_or_else(|| &PlainRenderer::default() as &dyn DiagnosticRenderer);
    renderer.render_batch(ctx.diagnostics(), &options, source)
}
```

---

## Snippet Layout Contract (Plain and Terminal)

For both plain and terminal renderers, snippet sections must follow this layout:

```
{severity}[{code}]: {title}
  --> {source_name}:{display_line}:{display_column}
   |
{line_num} | {source_line}
           | {underline with label}
   |
   = hint: {hint_text}
   = note: {note_text}

suggestion: {title} [{applicability}]
  {replacement_preview}
```

- Line numbers are right-aligned to the width of the widest number.
- Primary labels use `^` underlining; secondary labels use `-`.
- Multiple suggestions are listed sequentially, primary first.

---

## Implementation Notes

- `TerminalRenderer`, `PlainRenderer`, and `JsonRenderer` are zero-size structs
  using the default ANSI/plain/json strategies. Custom renderers come from the
  caller via `RendererSelector::add`.
- The `todo!` stubs in the trait implementations are placeholders; the actual
  formatting logic lives in internal submodules (`terminal`, `plain`, `json`)
  to keep the public types small.
- `DiagnosticsContext` does not own a renderer. Rendering is always a separate
  step performed by the caller using collected diagnostics.
- Tests must use `PlainRenderer` with `FixedEnvironmentDetector::redirected_ci()`
  for stable snapshot assertions.
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