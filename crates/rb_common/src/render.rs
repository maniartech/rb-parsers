use crate::diagnostics::{Diagnostic, DiagnosticsContext};
use crate::env::{ColorPreference, EnvironmentDetector, EnvironmentSnapshot, RealEnvironmentDetector};
use crate::spans::SourceId;

// ── SourceStore trait ─────────────────────────────────────────────────────────

/// Access to original source text, keyed by `SourceId`.
pub trait SourceStore: Send + Sync {
    /// Returns the raw source text for `source_id`, if available.
    fn source_text(&self, source_id: SourceId) -> Option<&str>;
    /// Returns a human-readable display name (file path, URL, etc.) for `source_id`.
    fn display_name(&self, source_id: SourceId) -> Option<&str>;
}

// ── SnippetLines ──────────────────────────────────────────────────────────────

/// A window into source text that provides the lines around a primary diagnostic span.
pub struct SnippetLines {
    /// Human-readable name of the source file (used in the rendered header).
    pub source_name: String,
    /// The individual source lines included in the snippet, with context lines.
    pub lines: Vec<SourceLine>,
    /// Byte range of the primary span within the full source text.
    pub primary_byte_range: std::ops::Range<usize>,
}

/// A single source line as part of a [`SnippetLines`] window.
pub struct SourceLine {
    /// 1-based line number in the original file.
    pub line_number: usize,
    /// The text content of this line (no trailing newline).
    pub content: String,
    /// `true` if this line is padding context outside the primary span.
    pub is_context: bool,
}

impl SnippetLines {
    /// Extracts a source window around `primary_byte_range` with `context_lines` padding lines.
    ///
    /// Returns `None` if the source is not available in `store`.
    pub fn extract(
        store: &dyn SourceStore,
        source_id: SourceId,
        primary_byte_range: std::ops::Range<usize>,
        context_lines: usize,
    ) -> Option<Self> {
        let text = store.source_text(source_id)?;
        let source_name = store
            .display_name(source_id)
            .unwrap_or("<unknown>")
            .to_string();

        let line_starts: Vec<usize> = std::iter::once(0)
            .chain(
                text.char_indices()
                    .filter(|(_, c)| *c == '\n')
                    .map(|(i, _)| i + 1),
            )
            .collect();

        let first_line =
            line_starts.partition_point(|&s| s <= primary_byte_range.start).saturating_sub(1);
        let last_line =
            line_starts.partition_point(|&s| s < primary_byte_range.end).saturating_sub(1);
        let start_line = first_line.saturating_sub(context_lines);
        let end_line = (last_line + context_lines).min(line_starts.len().saturating_sub(1));

        let mut lines = Vec::new();
        for i in start_line..=end_line {
            let line_start = line_starts[i];
            let line_end = if i + 1 < line_starts.len() {
                line_starts[i + 1].saturating_sub(1)
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

// ── OutputFormat ──────────────────────────────────────────────────────────────

/// The output format requested from a renderer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    /// Choose automatically based on the environment.
    #[default]
    Auto,
    /// ANSI-coloured terminal output.
    Terminal,
    /// Plain text without colour or unicode box-drawing.
    Plain,
    /// Machine-readable JSON envelope.
    Json,
}

// ── RenderTarget ──────────────────────────────────────────────────────────────

/// The output destination a renderer should write to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RenderTarget {
    /// Standard output.
    #[default]
    Stdout,
    /// Standard error.
    Stderr,
    /// In-memory string (e.g. for tests).
    Memory,
    /// A named custom sink.
    Custom(&'static str),
}

// ── RenderOptions ─────────────────────────────────────────────────────────────

/// Rendering options that control format, colour, width, and snippet verbosity.
#[derive(Debug, Clone)]
pub struct RenderOptions {
    /// Output format (plain / terminal / JSON / auto).
    pub format: OutputFormat,
    /// Colour preference for terminal output.
    pub color: ColorPreference,
    /// Maximum column width for wrapping, or `None` for unlimited.
    pub width: Option<usize>,
    /// Whether to embed source snippets in the rendered output.
    pub show_snippets: bool,
    /// Whether to use unicode box-drawing characters.
    pub unicode: bool,
    /// Number of context lines above and below the primary span.
    pub context_lines: usize,
    /// Pretty-print JSON output when `format == OutputFormat::Json`.
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
    /// Returns plain-text options with colour disabled.
    pub fn plain() -> Self {
        RenderOptions {
            format: OutputFormat::Plain,
            color: ColorPreference::Never,
            ..Default::default()
        }
    }

    /// Returns compact JSON options.
    pub fn json() -> Self {
        RenderOptions { format: OutputFormat::Json, ..Default::default() }
    }

    /// Returns pretty-printed JSON options.
    pub fn json_pretty() -> Self {
        RenderOptions { format: OutputFormat::Json, json_pretty: true, ..Default::default() }
    }
}

// ── RenderOutputPreset ────────────────────────────────────────────────────────

/// A set of predefined rendering configurations for common deployment scenarios.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderOutputPreset {
    /// Choose automatically — suitable for interactive CLI use.
    Auto,
    /// CI environment — plain text, no colour, no unicode.
    Ci,
    /// Machine consumer — compact JSON.
    Machine,
    /// Debug/development — rich terminal output with extra context lines.
    Debug,
}

impl RenderOutputPreset {
    /// Converts this preset into a concrete [`RenderOptions`].
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

// ── RenderRequest ─────────────────────────────────────────────────────────────

/// Parameters passed to [`DiagnosticRenderer::suitability`] so renderers can
/// decide whether they are appropriate for the current environment.
pub struct RenderRequest<'a> {
    /// An explicit format override requested by the caller, if any.
    pub format_hint: Option<OutputFormat>,
    /// The detected rendering environment (TTY, colour support, etc.).
    pub environment: &'a EnvironmentSnapshot,
    /// The output stream the rendered text will be written to.
    pub target: RenderTarget,
    /// Rendering options in effect.
    pub options: &'a RenderOptions,
}

// ── RendererSuitability ───────────────────────────────────────────────────────

/// How well a renderer is suited for a given [`RenderRequest`].
///
/// Renderers are ranked and the best-suited one wins.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RendererSuitability {
    /// This renderer cannot produce output for the request.
    Unsupported,
    /// This renderer can produce output but is not the best choice.
    Fallback(u8),
    /// This renderer is a good match for the request.
    Preferred(u8),
}

// ── DiagnosticRenderer trait ──────────────────────────────────────────────────

/// A renderer that can format a [`Diagnostic`] into a human- or machine-readable string.
pub trait DiagnosticRenderer: Send + Sync {
    /// Returns how well this renderer fits the [`RenderRequest`].
    fn suitability(&self, request: &RenderRequest<'_>) -> RendererSuitability;

    /// Renders a single diagnostic to a string.
    fn render(
        &self,
        diagnostic: &Diagnostic,
        options: &RenderOptions,
        source: Option<&dyn SourceStore>,
    ) -> String;

    /// Renders a slice of diagnostics, joining them with newlines.
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

// ── PlainRenderer ─────────────────────────────────────────────────────────────

/// A plain-text renderer that uses no ANSI escape codes.
#[derive(Debug, Default)]
pub struct PlainRenderer;

impl DiagnosticRenderer for PlainRenderer {
    fn suitability(&self, request: &RenderRequest<'_>) -> RendererSuitability {
        match request.format_hint.or(Some(request.options.format)) {
            Some(OutputFormat::Plain) => RendererSuitability::Preferred(200),
            Some(OutputFormat::Json) => RendererSuitability::Unsupported,
            _ => RendererSuitability::Fallback(50),
        }
    }

    fn render(
        &self,
        diagnostic: &Diagnostic,
        _options: &RenderOptions,
        source: Option<&dyn SourceStore>,
    ) -> String {
        use crate::spans::DiagnosticLocation;

        let mut out = String::new();

        // Header: severity[code]: message
        out.push_str(&format!(
            "{}[{}]: {}\n",
            diagnostic.severity,
            diagnostic.code,
            diagnostic.message
        ));

        // Primary label location
        if let Some(location) = diagnostic.primary_location() {
            let pos = location.start_position();
            let source_name = match location {
                DiagnosticLocation::Real(span) => {
                    source
                        .and_then(|s| s.display_name(span.source_id))
                        .unwrap_or("<unknown>")
                        .to_string()
                }
                DiagnosticLocation::InsertionPoint { source_id, .. }
                | DiagnosticLocation::EndOfFile { source_id, .. } => {
                    source
                        .and_then(|s| s.display_name(*source_id))
                        .unwrap_or("<unknown>")
                        .to_string()
                }
            };
            out.push_str(&format!(
                "  --> {}:{}:{}\n",
                source_name,
                pos.display_line(),
                pos.display_column()
            ));

            // Snippet
            if let (Some(store), DiagnosticLocation::Real(span)) = (source, location) {
                if let Some(snippet) = SnippetLines::extract(
                    store,
                    span.source_id,
                    span.byte_range(),
                    1,
                ) {
                    let max_num_w = snippet
                        .lines
                        .last()
                        .map(|l| format!("{}", l.line_number).len())
                        .unwrap_or(1);
                    for line in &snippet.lines {
                        let num_str = format!("{}", line.line_number);
                        let pad = max_num_w - num_str.len();
                        let prefix = if line.is_context { ' ' } else { '>' };
                        out.push_str(&format!(
                            "{} {:pad$}{} | {}\n",
                            prefix,
                            "",
                            num_str,
                            line.content,
                            pad = pad
                        ));
                    }
                }
            }
        }

        // Notes
        for note in &diagnostic.notes {
            out.push_str(&format!("   = note: {note}\n"));
        }

        // Hints
        for hint in &diagnostic.hints {
            out.push_str(&format!("   = hint: {}\n", hint.text));
        }

        // Suggestions
        for suggestion in &diagnostic.suggestions {
            out.push_str(&format!(
                "\nsuggestion: {} [{:?}]\n",
                suggestion.title, suggestion.applicability
            ));
        }

        out
    }
}

// ── TerminalRenderer ──────────────────────────────────────────────────────────

/// A terminal renderer that uses ANSI escape codes when the output is a TTY.
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
        let explicit_terminal = request
            .format_hint
            .or(Some(request.options.format))
            .is_some_and(|f| f == OutputFormat::Terminal);

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
        source: Option<&dyn SourceStore>,
    ) -> String {
        // For now delegate to plain; full ANSI implementation is out of scope
        // for the initial framework version.
        PlainRenderer.render(diagnostic, options, source)
    }
}

// ── JsonRenderer ──────────────────────────────────────────────────────────────

/// A renderer that serialises diagnostics as a JSON envelope.
#[derive(Debug, Default)]
pub struct JsonRenderer;

impl JsonRenderer {
    fn render_location_json(loc: &crate::spans::DiagnosticLocation) -> String {
        use crate::spans::DiagnosticLocation;
        match loc {
            DiagnosticLocation::Real(span) => {
                format!(
                    r#"{{"kind":"real","source_id":{},"start":{{"byte_offset":{},"line":{},"column":{}}},"end":{{"byte_offset":{},"line":{},"column":{}}}}}"#,
                    span.source_id.0,
                    span.start.byte_offset, span.start.line, span.start.column,
                    span.end.byte_offset, span.end.line, span.end.column
                )
            }
            DiagnosticLocation::InsertionPoint { source_id, at } => {
                format!(
                    r#"{{"kind":"insertion_point","source_id":{},"at":{{"byte_offset":{},"line":{},"column":{}}}}}"#,
                    source_id.0, at.byte_offset, at.line, at.column
                )
            }
            DiagnosticLocation::EndOfFile { source_id, at } => {
                format!(
                    r#"{{"kind":"end_of_file","source_id":{},"at":{{"byte_offset":{},"line":{},"column":{}}}}}"#,
                    source_id.0, at.byte_offset, at.line, at.column
                )
            }
        }
    }
}

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
        let labels_json: Vec<String> = diagnostic
            .labels
            .iter()
            .map(|l| {
                let style = match l.style {
                    crate::spans::LabelStyle::Primary => "primary",
                    crate::spans::LabelStyle::Secondary => "secondary",
                    crate::spans::LabelStyle::Context => "context",
                };
                let msg = match &l.message {
                    Some(m) => format!(r#""{m}""#),
                    None => "null".to_string(),
                };
                format!(
                    r#"{{"style":"{style}","location":{},"message":{msg}}}"#,
                    Self::render_location_json(&l.location)
                )
            })
            .collect();

        let hints_json: Vec<String> = diagnostic
            .hints
            .iter()
            .map(|h| format!(r#"{{"text":"{}","is_auto":{}}}"#, h.text.replace('"', "\\\""), h.is_auto))
            .collect();

        let notes_json: Vec<String> = diagnostic
            .notes
            .iter()
            .map(|n| format!(r#""{}""#, n.replace('"', "\\\"")))
            .collect();

        let diagnostic_json = format!(
            r#"{{"severity":"{severity}","code":"{code}","title":"{title}","message":"{message}","labels":[{labels}],"context":null,"notes":[{notes}],"hints":[{hints}],"suggestions":[]}}"#,
            severity = diagnostic.severity,
            code = diagnostic.code,
            title = diagnostic.title.replace('"', "\\\""),
            message = diagnostic.message.replace('"', "\\\""),
            labels = labels_json.join(","),
            notes = notes_json.join(","),
            hints = hints_json.join(","),
        );

        let wrapper =
            format!(r#"{{"schema_version":1,"diagnostics":[{diagnostic_json}]}}"#);

        if options.json_pretty {
            // Minimal indentation (2-space) without serde_json dependency
            pretty_print_json(&wrapper)
        } else {
            wrapper
        }
    }
}

/// Minimal JSON pretty-printer; not a general-purpose implementation.
fn pretty_print_json(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    let mut indent = 0usize;
    let mut in_string = false;
    let mut prev = '\0';
    for ch in s.chars() {
        if in_string {
            out.push(ch);
            if ch == '"' && prev != '\\' {
                in_string = false;
            }
        } else {
            match ch {
                '"' => { out.push(ch); in_string = true; }
                '{' | '[' => {
                    out.push(ch);
                    out.push('\n');
                    indent += 1;
                    out.extend(std::iter::repeat_n(' ', indent * 2));
                }
                '}' | ']' => {
                    out.push('\n');
                    indent = indent.saturating_sub(1);
                    out.extend(std::iter::repeat_n(' ', indent * 2));
                    out.push(ch);
                }
                ',' => {
                    out.push(ch);
                    out.push('\n');
                    out.extend(std::iter::repeat_n(' ', indent * 2));
                }
                ':' => { out.push(':'); out.push(' '); }
                ' ' | '\n' | '\t' => {}
                _ => { out.push(ch); }
            }
        }
        prev = ch;
    }
    out
}

// ── RendererSelector ──────────────────────────────────────────────────────────

/// Chooses the best available renderer for a given [`RenderRequest`].
pub struct RendererSelector {
    renderers: Vec<Box<dyn DiagnosticRenderer>>,
}

impl RendererSelector {
    /// Creates an empty selector with no registered renderers.
    pub fn new() -> Self {
        RendererSelector { renderers: Vec::new() }
    }

    /// Creates a selector pre-populated with the standard renderers
    /// (`TerminalRenderer`, `PlainRenderer`, `JsonRenderer`).
    pub fn default_renderers() -> Self {
        let mut s = RendererSelector::new();
        s.renderers.push(Box::new(TerminalRenderer));
        s.renderers.push(Box::new(PlainRenderer));
        s.renderers.push(Box::new(JsonRenderer));
        s
    }

    /// Adds `renderer` to this selector and returns `self` (builder pattern).
    pub fn with(mut self, renderer: Box<dyn DiagnosticRenderer>) -> Self {
        self.renderers.push(renderer);
        self
    }

    /// Returns the best-suited renderer for `request`, or `None` if all renderers
    /// reported [`RendererSuitability::Unsupported`].
    pub fn select<'a>(&'a self, request: &RenderRequest<'_>) -> Option<&'a dyn DiagnosticRenderer> {
        self.renderers
            .iter()
            .filter_map(|r| {
                let s = r.suitability(request);
                if s == RendererSuitability::Unsupported { None } else { Some((s, r.as_ref())) }
            })
            .max_by_key(|(s, _)| *s)
            .map(|(_, r)| r)
    }
}

impl Default for RendererSelector {
    fn default() -> Self { Self::new() }
}

/// Type alias for [`RendererSelector`]. Use this name when referring to the
/// registry pattern — both names refer to the same struct.
pub type RendererRegistry = RendererSelector;

// ── render_to_string ──────────────────────────────────────────────────────────

/// Renders a single `diagnostic` to a string using the best available renderer.
pub fn render_to_string(
    diagnostic: &Diagnostic,
    options: &RenderOptions,
    env: &EnvironmentSnapshot,
    target: RenderTarget,
    source: Option<&dyn SourceStore>,
) -> String {
    let request = RenderRequest { format_hint: None, environment: env, target, options };
    let selector = RendererSelector::default_renderers();
    match selector.select(&request) {
        Some(renderer) => renderer.render(diagnostic, options, source),
        None => PlainRenderer.render(diagnostic, options, source),
    }
}

// ── render_diagnostics ────────────────────────────────────────────────────────

/// Renders all diagnostics in `ctx` using `preset` and returns the combined string.
pub fn render_diagnostics(
    ctx: &DiagnosticsContext,
    preset: RenderOutputPreset,
    source: Option<&dyn SourceStore>,
) -> String {
    let options = preset.to_options();
    let env = RealEnvironmentDetector.detect();
    let request = RenderRequest {
        format_hint: None,
        environment: &env,
        target: RenderTarget::Memory,
        options: &options,
    };
    let selector = RendererSelector::default_renderers();
    let plain = PlainRenderer;
    let renderer: &dyn DiagnosticRenderer =
        selector.select(&request).unwrap_or(&plain);
    renderer.render_batch(ctx.diagnostics(), &options, source)
}
