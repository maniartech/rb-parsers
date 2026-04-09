use crate::spans::SourceSpan;

// ── EditKind ─────────────────────────────────────────────────────────────────

/// The structural kind of a single text edit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditKind {
    /// Insert `replacement` before `span.start`; `span` should be empty
    /// (`byte_len() == 0`) for a pure insertion.
    Insert,
    /// Replace the bytes covered by `span` with `replacement`.
    Replace,
    /// Delete the bytes covered by `span`; `replacement` must be empty.
    Delete,
}

// ── Applicability ─────────────────────────────────────────────────────────────

/// Confidence level that determines whether a tool may apply a suggestion
/// automatically.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Applicability {
    /// Safe to apply automatically without human review.
    MachineApplicable,
    /// Likely correct but may need case-by-case review.
    MaybeApplicable,
    /// Requires human judgment; display-only.
    Manual,
}

// ── TextEdit ──────────────────────────────────────────────────────────────────

/// A single atomic source edit.
///
/// For pure insertions the span should have `byte_len() == 0`, locating the
/// insertion point precisely.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEdit {
    /// The source region to be modified.
    pub span: SourceSpan,
    /// The replacement text. Empty string for `Delete` edits.
    pub replacement: String,
    /// The structural intent of this edit.
    pub kind: EditKind,
}

impl TextEdit {
    pub fn insert(span: SourceSpan, text: impl Into<String>) -> Self {
        TextEdit { span, replacement: text.into(), kind: EditKind::Insert }
    }

    pub fn replace(span: SourceSpan, text: impl Into<String>) -> Self {
        TextEdit { span, replacement: text.into(), kind: EditKind::Replace }
    }

    pub fn delete(span: SourceSpan) -> Self {
        TextEdit { span, replacement: String::new(), kind: EditKind::Delete }
    }
}

// ── Suggestion ────────────────────────────────────────────────────────────────

/// A logical fix grouping one or more coordinated text edits.
///
/// Multiple `TextEdit` values under one `Suggestion` represent changes that must
/// be applied together. When multiple independent fixes exist for one diagnostic,
/// they are represented as multiple `Suggestion` values on the same `Diagnostic`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suggestion {
    /// Short human-readable description of the fix.
    pub title: String,
    /// The edits to apply. May be multiple coordinated changes.
    pub edits: Vec<TextEdit>,
    /// Whether this suggestion can be applied automatically.
    pub applicability: Applicability,
    /// Marks this as the preferred suggestion when multiple alternatives exist.
    pub is_primary: bool,
}

impl Suggestion {
    pub fn new(
        title: impl Into<String>,
        edits: Vec<TextEdit>,
        applicability: Applicability,
    ) -> Self {
        Suggestion {
            title: title.into(),
            edits,
            applicability,
            is_primary: false,
        }
    }

    pub fn machine_applicable(title: impl Into<String>, edits: Vec<TextEdit>) -> Self {
        Suggestion {
            title: title.into(),
            edits,
            applicability: Applicability::MachineApplicable,
            is_primary: false,
        }
    }

    pub fn mark_primary(mut self) -> Self {
        self.is_primary = true;
        self
    }

    /// Returns true when all edits are safe to apply automatically.
    pub fn is_machine_applicable(&self) -> bool {
        self.applicability == Applicability::MachineApplicable
    }
}
