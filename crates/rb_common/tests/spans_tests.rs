use rb_common::spans::{
    DiagnosticLocation, LabelStyle, SourceId, SourcePosition, SourceSpan, SpanLabel,
};

// ── SourceId ──────────────────────────────────────────────────────────────────

#[test]
fn source_id_unknown_is_zero() {
    assert_eq!(SourceId::UNKNOWN, SourceId(0));
    assert!(SourceId::UNKNOWN.is_unknown());
}

#[test]
fn source_id_nonzero_is_not_unknown() {
    assert!(!SourceId(1).is_unknown());
    assert!(!SourceId(u32::MAX).is_unknown());
}

#[test]
fn source_id_equality() {
    assert_eq!(SourceId(5), SourceId(5));
    assert_ne!(SourceId(5), SourceId(6));
}

// ── SourcePosition ────────────────────────────────────────────────────────────

#[test]
fn source_position_display_is_one_based() {
    let pos = SourcePosition::new(0, 0, 0);
    assert_eq!(pos.display_line(), 1);
    assert_eq!(pos.display_column(), 1);
}

#[test]
fn source_position_display_middle_of_file() {
    let pos = SourcePosition::new(42, 4, 7);
    assert_eq!(pos.display_line(), 5);
    assert_eq!(pos.display_column(), 8);
}

#[test]
fn source_position_zero_const() {
    let z = SourcePosition::ZERO;
    assert_eq!(z.byte_offset, 0);
    assert_eq!(z.line, 0);
    assert_eq!(z.column, 0);
}

// ── SourceSpan ────────────────────────────────────────────────────────────────

fn span(src: u32, start: usize, end: usize) -> SourceSpan {
    SourceSpan::new(
        SourceId(src),
        SourcePosition::new(start, 0, start),
        SourcePosition::new(end, 0, end),
    )
}

#[test]
fn source_span_byte_len() {
    assert_eq!(span(1, 0, 5).byte_len(), 5);
    assert_eq!(span(1, 3, 3).byte_len(), 0);
}

#[test]
fn source_span_is_empty() {
    assert!(span(1, 7, 7).is_empty());
    assert!(!span(1, 7, 8).is_empty());
}

#[test]
fn source_span_byte_range() {
    assert_eq!(span(1, 2, 8).byte_range(), 2..8);
}

#[test]
fn source_span_unknown_const() {
    assert_eq!(SourceSpan::UNKNOWN.source_id, SourceId::UNKNOWN);
    assert_eq!(SourceSpan::UNKNOWN.byte_len(), 0);
}

#[test]
fn source_span_empty_at() {
    let pos = SourcePosition::new(10, 1, 5);
    let s = SourceSpan::empty_at(SourceId(2), pos);
    assert!(s.is_empty());
    assert_eq!(s.source_id, SourceId(2));
    assert_eq!(s.start.byte_offset, 10);
}

// ── overlaps ──────────────────────────────────────────────────────────────────

#[test]
fn overlaps_true_for_partial_overlap() {
    assert!(span(1, 0, 10).overlaps(span(1, 5, 15)));
}

#[test]
fn overlaps_true_for_contained_span() {
    assert!(span(1, 0, 20).overlaps(span(1, 5, 10)));
}

#[test]
fn overlaps_false_for_adjacent_spans() {
    // [0,5) and [5,10) are adjacent — not overlapping
    assert!(!span(1, 0, 5).overlaps(span(1, 5, 10)));
}

#[test]
fn overlaps_false_when_different_sources() {
    assert!(!span(1, 0, 10).overlaps(span(2, 0, 10)));
}

// ── merge ─────────────────────────────────────────────────────────────────────

#[test]
fn merge_takes_outer_bounds() {
    let merged = span(1, 2, 5).merge(span(1, 4, 9)).unwrap();
    assert_eq!(merged.start.byte_offset, 2);
    assert_eq!(merged.end.byte_offset, 9);
}

#[test]
fn merge_fails_across_sources() {
    assert!(span(1, 0, 5).merge(span(2, 0, 5)).is_none());
}

#[test]
fn merge_same_span_is_idempotent() {
    let s = span(1, 3, 7);
    let m = s.merge(s).unwrap();
    assert_eq!(m, s);
}

// ── DiagnosticLocation ────────────────────────────────────────────────────────

#[test]
fn diagnostic_location_real_source_id() {
    let loc = DiagnosticLocation::real(span(5, 0, 3));
    assert_eq!(loc.source_id(), SourceId(5));
}

#[test]
fn diagnostic_location_insertion_point() {
    let loc = DiagnosticLocation::InsertionPoint {
        source_id: SourceId(3),
        at: SourcePosition::new(10, 1, 0),
    };
    assert_eq!(loc.source_id(), SourceId(3));
    assert_eq!(loc.start_position().byte_offset, 10);
}

#[test]
fn diagnostic_location_eof() {
    let loc = DiagnosticLocation::EndOfFile {
        source_id: SourceId(7),
        at: SourcePosition::new(99, 9, 0),
    };
    assert_eq!(loc.source_id(), SourceId(7));
    assert_eq!(loc.start_position().line, 9);
}

#[test]
fn diagnostic_location_from_span() {
    let s = span(1, 0, 5);
    let loc: DiagnosticLocation = s.into();
    assert!(matches!(loc, DiagnosticLocation::Real(_)));
}

// ── SpanLabel ─────────────────────────────────────────────────────────────────

#[test]
fn span_label_primary_no_message() {
    let loc = DiagnosticLocation::real(span(1, 0, 3));
    let label = SpanLabel::primary(loc);
    assert_eq!(label.style, LabelStyle::Primary);
    assert!(label.message.is_none());
}

#[test]
fn span_label_secondary_with_message() {
    let loc = DiagnosticLocation::real(span(1, 0, 3));
    let label = SpanLabel::secondary_with_message(loc, "here");
    assert_eq!(label.style, LabelStyle::Secondary);
    assert_eq!(label.message.as_deref(), Some("here"));
}
