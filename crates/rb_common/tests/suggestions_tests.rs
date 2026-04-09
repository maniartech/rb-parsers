use rb_common::suggestions::{Applicability, EditKind, Suggestion, TextEdit};
use rb_common::spans::{SourceId, SourcePosition, SourceSpan};

fn span(start: usize, end: usize) -> SourceSpan {
    SourceSpan::new(
        SourceId(1),
        SourcePosition::new(start, 0, start),
        SourcePosition::new(end, 0, end),
    )
}

// ── TextEdit ──────────────────────────────────────────────────────────────────

#[test]
fn text_edit_insert() {
    let e = TextEdit::insert(span(5, 5), ";");
    assert_eq!(e.kind, EditKind::Insert);
    assert_eq!(e.replacement, ";");
    assert!(e.span.is_empty());
}

#[test]
fn text_edit_replace() {
    let e = TextEdit::replace(span(0, 3), "let");
    assert_eq!(e.kind, EditKind::Replace);
    assert_eq!(e.replacement, "let");
    assert_eq!(e.span.byte_len(), 3);
}

#[test]
fn text_edit_delete() {
    let e = TextEdit::delete(span(2, 7));
    assert_eq!(e.kind, EditKind::Delete);
    assert_eq!(e.replacement, "");
    assert_eq!(e.span.byte_len(), 5);
}

// ── Applicability ordering ────────────────────────────────────────────────────

#[test]
fn applicability_ordering() {
    use Applicability::*;
    // Declaration order: MachineApplicable < MaybeApplicable < Manual
    assert!(MachineApplicable < MaybeApplicable);
    assert!(MaybeApplicable < Manual);
}

// ── Suggestion ────────────────────────────────────────────────────────────────

#[test]
fn suggestion_new_fields() {
    let s = Suggestion::new(
        "insert semicolon",
        vec![TextEdit::insert(span(5, 5), ";")],
        Applicability::MachineApplicable,
    );
    assert_eq!(s.title, "insert semicolon");
    assert_eq!(s.applicability, Applicability::MachineApplicable);
    assert!(!s.is_primary);
    assert_eq!(s.edits.len(), 1);
}

#[test]
fn suggestion_machine_applicable_shorthand() {
    let s = Suggestion::machine_applicable(
        "fix it",
        vec![TextEdit::delete(span(0, 1))],
    );
    assert_eq!(s.applicability, Applicability::MachineApplicable);
}

#[test]
fn suggestion_with_is_primary_flag() {
    let s = Suggestion {
        title: "primary fix".to_string(),
        edits: vec![],
        applicability: Applicability::Manual,
        is_primary: true,
    };
    assert!(s.is_primary);
}

#[test]
fn suggestion_multi_edit() {
    let s = Suggestion::new(
        "multi fix",
        vec![
            TextEdit::replace(span(0, 3), "var"),
            TextEdit::insert(span(10, 10), ";"),
        ],
        Applicability::MaybeApplicable,
    );
    assert_eq!(s.edits.len(), 2);
    assert_eq!(s.edits[0].kind, EditKind::Replace);
    assert_eq!(s.edits[1].kind, EditKind::Insert);
}
