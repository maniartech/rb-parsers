use rb_common::catalog::{ErrorCode, ErrorSeverity};
use rb_common::diagnostics::{
    CollectingSink, CompositeSink, DiagnosticBuilder, DiagnosticsContext, DiagnosticsMode,
    Hint, HookSink, NullSink, SeverityPolicy,
};
use rb_common::spans::{DiagnosticLocation, SourceId, SourcePosition, SourceSpan};

// ── Shared helper ─────────────────────────────────────────────────────────────

fn simple_loc() -> DiagnosticLocation {
    DiagnosticLocation::real(SourceSpan::new(
        SourceId(1),
        SourcePosition::new(0, 0, 0),
        SourcePosition::new(5, 0, 5),
    ))
}

fn error_diag(msg: &str) -> rb_common::diagnostics::Diagnostic {
    DiagnosticBuilder::new(
        ErrorCode("TST-001"),
        ErrorSeverity::Error,
        "test error",
        msg,
    )
    .build()
}

fn warning_diag(msg: &str) -> rb_common::diagnostics::Diagnostic {
    DiagnosticBuilder::new(
        ErrorCode("TST-002"),
        ErrorSeverity::Warning,
        "test warning",
        msg,
    )
    .build()
}

// ── Hint ──────────────────────────────────────────────────────────────────────

#[test]
fn hint_authored_is_not_auto() {
    let h = Hint::authored("check braces");
    assert!(!h.is_auto);
    assert_eq!(h.text, "check braces");
}

#[test]
fn hint_auto_generated_is_auto() {
    let h = Hint::auto_generated("missing semicolon");
    assert!(h.is_auto);
}

// ── DiagnosticBuilder ─────────────────────────────────────────────────────────

#[test]
fn builder_new_creates_correct_fields() {
    let d = DiagnosticBuilder::new(
        ErrorCode("X-001"),
        ErrorSeverity::Error,
        "my title",
        "my message",
    )
    .build();

    assert_eq!(d.code, ErrorCode("X-001"));
    assert_eq!(d.severity, ErrorSeverity::Error);
    assert_eq!(d.title, "my title");
    assert_eq!(d.message, "my message");
    assert!(d.labels.is_empty());
    assert!(d.hints.is_empty());
    assert!(d.notes.is_empty());
}

#[test]
fn builder_primary_label_is_first() {
    let d = DiagnosticBuilder::new(ErrorCode("X"), ErrorSeverity::Error, "t", "m")
        .primary(simple_loc())
        .build();
    assert_eq!(d.labels.len(), 1);
    assert_eq!(d.labels[0].style, rb_common::spans::LabelStyle::Primary);
}

#[test]
fn builder_secondary_label_appended() {
    let d = DiagnosticBuilder::new(ErrorCode("X"), ErrorSeverity::Error, "t", "m")
        .primary(simple_loc())
        .secondary(simple_loc())
        .build();
    assert_eq!(d.labels.len(), 2);
    assert_eq!(d.labels[1].style, rb_common::spans::LabelStyle::Secondary);
}

#[test]
fn builder_notes_and_hints() {
    let d = DiagnosticBuilder::new(ErrorCode("X"), ErrorSeverity::Error, "t", "m")
        .note("fact one")
        .hint("try this")
        .build();
    assert_eq!(d.notes, ["fact one"]);
    assert_eq!(d.hints[0].text, "try this");
    assert!(!d.hints[0].is_auto);
}

#[test]
fn builder_severity_override() {
    let d = DiagnosticBuilder::new(ErrorCode("X"), ErrorSeverity::Error, "t", "m")
        .severity(ErrorSeverity::Warning)
        .build();
    assert_eq!(d.severity, ErrorSeverity::Warning);
}

#[test]
fn diagnostic_is_error_and_is_warning() {
    let e = error_diag("foo");
    assert!(e.is_error());
    assert!(!e.is_warning());

    let w = warning_diag("bar");
    assert!(!w.is_error());
    assert!(w.is_warning());
}

#[test]
fn diagnostic_primary_location_found() {
    let d = DiagnosticBuilder::new(ErrorCode("X"), ErrorSeverity::Error, "t", "m")
        .primary(simple_loc())
        .build();
    assert!(d.primary_location().is_some());
}

#[test]
fn diagnostic_primary_location_absent_without_primary_label() {
    let d = DiagnosticBuilder::new(ErrorCode("X"), ErrorSeverity::Error, "t", "m")
        .secondary(simple_loc())
        .build();
    assert!(d.primary_location().is_none());
}

// ── NullSink ──────────────────────────────────────────────────────────────────

#[test]
fn null_sink_accepts_without_panic() {
    let sink = NullSink;
    use rb_common::diagnostics::DiagnosticSink;
    sink.emit(&error_diag("test")); // must not panic
}

// ── CollectingSink ────────────────────────────────────────────────────────────

#[test]
fn collecting_sink_count() {
    let sink = CollectingSink::new();
    use rb_common::diagnostics::DiagnosticSink;
    sink.emit(&error_diag("a"));
    sink.emit(&warning_diag("b"));
    assert_eq!(sink.count(), 2);
    assert_eq!(sink.error_count(), 1);
}

#[test]
fn collecting_sink_snapshot_does_not_drain() {
    let sink = CollectingSink::new();
    use rb_common::diagnostics::DiagnosticSink;
    sink.emit(&error_diag("x"));
    let _ = sink.snapshot();
    assert_eq!(sink.count(), 1); // still there
}

#[test]
fn collecting_sink_take_all_drains() {
    let sink = CollectingSink::new();
    use rb_common::diagnostics::DiagnosticSink;
    sink.emit(&error_diag("x"));
    let taken = sink.take_all();
    assert_eq!(taken.len(), 1);
    assert_eq!(sink.count(), 0);
}

// ── HookSink ─────────────────────────────────────────────────────────────────

#[test]
fn hook_sink_calls_closure() {
    use std::sync::{Arc, Mutex};
    let fired = Arc::new(Mutex::new(0usize));
    let fired2 = Arc::clone(&fired);

    let sink = HookSink::new(move |_: &rb_common::diagnostics::Diagnostic| {
        *fired2.lock().unwrap() += 1;
    });
    use rb_common::diagnostics::DiagnosticSink;
    sink.emit(&error_diag("x"));
    sink.emit(&error_diag("y"));
    assert_eq!(*fired.lock().unwrap(), 2);
}

// ── CompositeSink ─────────────────────────────────────────────────────────────

#[test]
fn composite_sink_fans_out_to_all_sinks() {
    let s1 = CollectingSink::new();
    let s2 = CollectingSink::new();

    // Use raw pointers-to-collected to read after move.
    // Instead we use a channel approach via HookSink.
    use std::sync::{Arc, Mutex};
    let count = Arc::new(Mutex::new(0usize));
    let count2 = Arc::clone(&count);

    let composite = CompositeSink::new()
        .with(Box::new(NullSink))
        .with(Box::new(HookSink::new(move |_| {
            *count2.lock().unwrap() += 1;
        })));

    use rb_common::diagnostics::DiagnosticSink;
    composite.emit(&error_diag("a"));
    composite.emit(&error_diag("b"));
    assert_eq!(*count.lock().unwrap(), 2);
    drop(s1);
    drop(s2);
}

// ── DiagnosticsContext ────────────────────────────────────────────────────────

#[test]
fn context_collecting_stores_diagnostics() {
    let mut ctx = DiagnosticsContext::collecting();
    ctx.emit(error_diag("e1"));
    ctx.emit(warning_diag("w1"));
    assert_eq!(ctx.collected().len(), 2);
    assert_eq!(ctx.error_count(), 1);
    assert_eq!(ctx.warning_count(), 1);
    assert!(ctx.has_errors());
}

#[test]
fn context_null_discards_all() {
    let mut ctx = DiagnosticsContext::null();
    ctx.emit(error_diag("e1"));
    assert_eq!(ctx.collected().len(), 0);
    assert!(!ctx.has_errors()); // counts also not incremented
}

#[test]
fn context_emit_mode_does_not_retain() {
    let sink = CollectingSink::new();
    // We can't move+inspect sink after ownership transfer, so use Arc.
    use std::sync::Arc;
    let shared = Arc::new(CollectingSink::new());
    struct ArcSink(Arc<CollectingSink>);
    impl rb_common::diagnostics::DiagnosticSink for ArcSink {
        fn emit(&self, d: &rb_common::diagnostics::Diagnostic) { self.0.emit(d); }
    }
    let shared2 = Arc::clone(&shared);
    let mut ctx = DiagnosticsContext::emitting(Box::new(ArcSink(shared2)));
    ctx.emit(error_diag("x"));
    assert_eq!(ctx.collected().len(), 0); // not retained
    assert_eq!(shared.count(), 1);        // but went to sink
    drop(sink);
}

#[test]
fn context_collect_and_emit_both_paths() {
    use std::sync::Arc;
    let shared = Arc::new(CollectingSink::new());
    struct ArcSink(Arc<CollectingSink>);
    impl rb_common::diagnostics::DiagnosticSink for ArcSink {
        fn emit(&self, d: &rb_common::diagnostics::Diagnostic) { self.0.emit(d); }
    }
    let shared2 = Arc::clone(&shared);
    let mut ctx = DiagnosticsContext::collecting_and_emitting(Box::new(ArcSink(shared2)));
    ctx.emit(error_diag("y"));
    assert_eq!(ctx.collected().len(), 1);
    assert_eq!(shared.count(), 1);
}

#[test]
fn context_take_all_drains() {
    let mut ctx = DiagnosticsContext::collecting();
    ctx.emit(error_diag("a"));
    ctx.emit(error_diag("b"));
    let taken = ctx.take_all();
    assert_eq!(taken.len(), 2);
    assert_eq!(ctx.collected().len(), 0);
}

#[test]
fn context_by_code_filters_correctly() {
    let mut ctx = DiagnosticsContext::collecting();
    ctx.emit(error_diag("x"));
    ctx.emit(
        DiagnosticBuilder::new(ErrorCode("OTHER"), ErrorSeverity::Error, "t", "y").build(),
    );
    let found: Vec<_> = ctx.by_code(ErrorCode("TST-001")).collect();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].message, "x");
}

#[test]
fn context_errors_iterator() {
    let mut ctx = DiagnosticsContext::collecting();
    ctx.emit(error_diag("e"));
    ctx.emit(warning_diag("w"));
    let errors: Vec<_> = ctx.errors().collect();
    assert_eq!(errors.len(), 1);
    assert!(errors[0].is_error());
}

#[test]
fn context_is_collecting_flag() {
    assert!(DiagnosticsContext::collecting().is_collecting());
    assert!(!DiagnosticsContext::null().is_collecting());
}

// ── SeverityPolicy ────────────────────────────────────────────────────────────

#[test]
fn severity_policy_suppresses_code() {
    let policy = SeverityPolicy {
        suppressed: vec![ErrorCode("TST-001")],
        ..Default::default()
    };
    assert!(policy.apply(error_diag("x")).is_none());
    // other codes pass through
    assert!(policy.apply(warning_diag("y")).is_some());
}

#[test]
fn severity_policy_warnings_as_errors() {
    let policy = SeverityPolicy { warnings_as_errors: true, ..Default::default() };
    let result = policy.apply(warning_diag("w")).unwrap();
    assert_eq!(result.severity, ErrorSeverity::Error);
}

#[test]
fn severity_policy_override_replaces_severity() {
    let policy = SeverityPolicy {
        overrides: vec![(ErrorCode("TST-001"), ErrorSeverity::Info)],
        ..Default::default()
    };
    let result = policy.apply(error_diag("e")).unwrap();
    assert_eq!(result.severity, ErrorSeverity::Info);
}

#[test]
fn severity_policy_passthrough_when_empty() {
    let policy = SeverityPolicy::default();
    let result = policy.apply(error_diag("e")).unwrap();
    assert_eq!(result.severity, ErrorSeverity::Error);
}
