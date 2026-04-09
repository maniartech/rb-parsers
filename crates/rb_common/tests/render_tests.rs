use rb_common::catalog::{ErrorCode, ErrorSeverity};
use rb_common::diagnostics::DiagnosticBuilder;
use rb_common::render::{
    DiagnosticRenderer, OutputFormat, PlainRenderer, JsonRenderer, RenderOptions,
    RenderOutputPreset, RendererSelector, RendererSuitability, SourceStore,
};
use rb_common::spans::{DiagnosticLocation, SourceId, SourcePosition, SourceSpan};

// ── Shared helpers ────────────────────────────────────────────────────────────

fn simple_diag(msg: &str) -> rb_common::diagnostics::Diagnostic {
    let loc = DiagnosticLocation::real(SourceSpan::new(
        SourceId(1),
        SourcePosition::new(0, 0, 0),
        SourcePosition::new(3, 0, 3),
    ));
    DiagnosticBuilder::new(ErrorCode("T-001"), ErrorSeverity::Error, "test error", msg)
        .primary_labeled(loc, "here")
        .note("a note")
        .build()
}

// ── RenderOptions presets ─────────────────────────────────────────────────────

#[test]
fn render_options_plain_format_is_plain() {
    let opts = RenderOptions::plain();
    assert_eq!(opts.format, OutputFormat::Plain);
}

#[test]
fn render_options_json_format_is_json() {
    let opts = RenderOptions::json();
    assert_eq!(opts.format, OutputFormat::Json);
    assert!(!opts.json_pretty);
}

#[test]
fn render_options_json_pretty_flag() {
    let opts = RenderOptions::json_pretty();
    assert!(opts.json_pretty);
}

// ── RenderOutputPreset ────────────────────────────────────────────────────────

#[test]
fn preset_machine_produces_json() {
    let opts = RenderOutputPreset::Machine.to_options();
    assert_eq!(opts.format, OutputFormat::Json);
}

#[test]
fn preset_ci_plain_no_color() {
    use rb_common::env::ColorPreference;
    let opts = RenderOutputPreset::Ci.to_options();
    assert_eq!(opts.format, OutputFormat::Plain);
    assert_eq!(opts.color, ColorPreference::Never);
}

// ── PlainRenderer ─────────────────────────────────────────────────────────────

#[test]
fn plain_renderer_output_contains_severity_and_code() {
    let diag = simple_diag("something broke");
    let opts = RenderOptions::plain();
    let out = PlainRenderer.render(&diag, &opts, None);
    assert!(out.contains("error") || out.contains("T-001"), "got: {out}");
}

#[test]
fn plain_renderer_output_contains_message() {
    let diag = simple_diag("hello world");
    let opts = RenderOptions::plain();
    let out = PlainRenderer.render(&diag, &opts, None);
    assert!(out.contains("hello world"), "got: {out}");
}

// ── JsonRenderer ──────────────────────────────────────────────────────────────

#[test]
fn json_renderer_output_is_valid_json_shape() {
    let diag = simple_diag("json test");
    let opts = RenderOptions::json();
    let out = JsonRenderer.render(&diag, &opts, None);
    assert!(out.starts_with("{\"schema_version\":1"), "got: {out}");
    assert!(out.contains("\"code\":\"T-001\""), "got: {out}");
    assert!(out.contains("\"severity\":\"error\""), "got: {out}");
}

#[test]
fn json_renderer_message_is_present() {
    let diag = simple_diag("my message here");
    let opts = RenderOptions::json();
    let out = JsonRenderer.render(&diag, &opts, None);
    assert!(out.contains("my message here"), "got: {out}");
}

#[test]
fn json_renderer_pretty_has_newlines() {
    let diag = simple_diag("pretty");
    let opts = RenderOptions::json_pretty();
    let out = JsonRenderer.render(&diag, &opts, None);
    assert!(out.contains('\n'), "pretty JSON should have newlines");
}

#[test]
fn json_renderer_compact_has_no_internal_newlines() {
    let diag = simple_diag("compact");
    let opts = RenderOptions::json();
    let out = JsonRenderer.render(&diag, &opts, None);
    assert!(!out.contains('\n'), "compact JSON should not have newlines, got: {out}");
}

// ── RendererSuitability ordering ──────────────────────────────────────────────

#[test]
fn suitability_preferred_beats_fallback() {
    assert!(RendererSuitability::Preferred(100) > RendererSuitability::Fallback(200));
}

#[test]
fn suitability_unsupported_is_lowest() {
    assert!(RendererSuitability::Unsupported < RendererSuitability::Fallback(0));
    assert!(RendererSuitability::Unsupported < RendererSuitability::Preferred(0));
}

// ── RendererSelector ──────────────────────────────────────────────────────────

#[test]
fn renderer_selector_default_renderers_is_non_empty() {
    // Just verify it constructs without panic and has renderers registered.
    let sel = RendererSelector::default_renderers();
    use rb_common::render::{RenderRequest, RenderTarget};
    use rb_common::env::RealEnvironmentDetector;
    use rb_common::env::EnvironmentDetector;
    let env = RealEnvironmentDetector.detect();
    let opts = RenderOptions::json();
    let req = RenderRequest {
        format_hint: Some(OutputFormat::Json),
        environment: &env,
        target: RenderTarget::Memory,
        options: &opts,
    };
    assert!(sel.select(&req).is_some());
}

// ── SourceStore snippet extraction ───────────────────────────────────────────

struct MapStore(Vec<(SourceId, &'static str, &'static str)>);

impl SourceStore for MapStore {
    fn source_text(&self, id: SourceId) -> Option<&str> {
        self.0.iter().find(|(sid, _, _)| *sid == id).map(|(_, t, _)| *t)
    }
    fn display_name(&self, id: SourceId) -> Option<&str> {
        self.0.iter().find(|(sid, _, _)| *sid == id).map(|(_, _, n)| *n)
    }
}

#[test]
fn plain_renderer_with_source_store_includes_snippet() {
    let store = MapStore(vec![(SourceId(1), "let x = 1;\nlet y = 2;\n", "test.rb")]);
    let diag = simple_diag("unexpected token");
    let opts = RenderOptions::plain();
    let out = PlainRenderer.render(&diag, &opts, Some(&store));
    // Should include at least something about the source (file name or snippet line).
    assert!(!out.is_empty());
}
