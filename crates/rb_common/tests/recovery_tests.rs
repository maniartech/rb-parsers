use rb_common::recovery::{
    RecoveryBoundary, RecoveryBoundaryKind, RecoveryBoundarySet, RecoveryConfig, RecoveryMode,
};
use rb_common::spans::{SourceId, SourcePosition, SourceSpan};

fn span1() -> SourceSpan {
    SourceSpan::new(SourceId(1), SourcePosition::ZERO, SourcePosition::new(3, 0, 3))
}

// ── RecoveryMode / RecoveryConfig ─────────────────────────────────────────────

#[test]
fn recovery_config_default_values() {
    let cfg = RecoveryConfig::default();
    assert_eq!(cfg.mode, RecoveryMode::ContinueBounded);
    assert_eq!(cfg.max_errors, 20);
    assert_eq!(cfg.max_recovery_skips, 50);
}

#[test]
fn recovery_config_fail_fast() {
    let cfg = RecoveryConfig::fail_fast();
    assert_eq!(cfg.mode, RecoveryMode::FailFast);
    assert_eq!(cfg.max_errors, 1);
    assert_eq!(cfg.max_recovery_skips, 0);
}

// ── RecoveryBoundary ──────────────────────────────────────────────────────────

#[test]
fn recovery_boundary_new_has_no_span() {
    let b = RecoveryBoundary::new(RecoveryBoundaryKind::Separator);
    assert!(b.span.is_none());
    assert_eq!(b.kind, RecoveryBoundaryKind::Separator);
}

#[test]
fn recovery_boundary_with_span() {
    let b = RecoveryBoundary::with_span(RecoveryBoundaryKind::ClosingDelimiter, span1());
    assert!(b.span.is_some());
    assert_eq!(b.kind, RecoveryBoundaryKind::ClosingDelimiter);
}

// ── RecoveryBoundarySet ───────────────────────────────────────────────────────

#[test]
fn boundary_set_common_is_not_empty() {
    let set = RecoveryBoundarySet::common();
    assert!(!set.is_empty());
}

#[test]
fn boundary_set_with_kind_appends() {
    let set = RecoveryBoundarySet::common()
        .with_kind(RecoveryBoundaryKind::Custom("EOS"));
    assert!(set.kinds.contains(&RecoveryBoundaryKind::Custom("EOS")));
}

#[test]
fn boundary_set_with_block_boundaries_includes_block_boundary() {
    let set = RecoveryBoundarySet::with_block_boundaries();
    assert!(set.kinds.contains(&RecoveryBoundaryKind::BlockBoundary));
}

#[test]
fn boundary_set_empty_default() {
    let set = RecoveryBoundarySet::default();
    assert!(set.is_empty());
}

#[test]
fn boundary_kind_custom_equality() {
    assert_eq!(
        RecoveryBoundaryKind::Custom("X"),
        RecoveryBoundaryKind::Custom("X")
    );
    assert_ne!(
        RecoveryBoundaryKind::Custom("X"),
        RecoveryBoundaryKind::Custom("Y")
    );
}
