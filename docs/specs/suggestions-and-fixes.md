# Spec: Suggestions and Fixes

**Status**: Ready for implementation
**Module**: `rb_common::suggestions`
**Depends on**: `rb_common::spans`
**Requirement source**: `docs/requirements/suggestions-and-fixes.md`

---

## Decisions Made

| Question | Decision |
|---|---|
| Zero-length insertion edits | Use `DiagnosticLocation::InsertionPoint` for the span; `EditKind::Insert` with empty replacement is also acceptable for span-less inline cases |
| Diff/patch previews | Deferred; renderers receive `TextEdit` fields and build their own preview if needed |
| Preferred suggestion marker | `Suggestion` has an `is_primary: bool` field so tooling can distinguish the recommended fix from alternatives |

---

## Module Layout

```
rb_common::suggestions
├── EditKind
├── Applicability
├── TextEdit
└── Suggestion
```

---

## Types

### EditKind

The structural kind of a single text edit.

```rust
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
```

---

### Applicability

Confidence level that determines whether a tool may apply a suggestion
automatically.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Applicability {
    /// Safe to apply automatically without human review.
    MachineApplicable,
    /// Likely correct but may need case-by-case review.
    MaybeApplicable,
    /// Requires human judgment; display-only.
    Manual,
}
```

---

### TextEdit

A single atomic source edit.

`span` uses `SourceSpan` from `rb_common::spans`. For pure insertions the span
should have `byte_len() == 0`, locating the insertion point precisely.

```rust
use crate::spans::SourceSpan;

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
```

---

### Suggestion

A logical fix grouping one or more coordinated text edits.

Multiple `TextEdit` values under one `Suggestion` represent changes that must
be applied together. When multiple independent fixes exist for one diagnostic,
they are represented as multiple `Suggestion` values attached to the same
`Diagnostic`.

```rust
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
    pub fn new(title: impl Into<String>, edits: Vec<TextEdit>, applicability: Applicability) -> Self {
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
```

---

## Serialization Shape (JSON)

When serialized by the JSON renderer, each `Suggestion` should produce:

```json
{
  "title": "Insert closing '}'",
  "applicability": "machine_applicable",
  "is_primary": true,
  "edits": [
    {
      "kind": "insert",
      "span": {
        "source_id": 1,
        "start": { "byte_offset": 230, "line": 14, "column": 0 },
        "end":   { "byte_offset": 230, "line": 14, "column": 0 }
      },
      "replacement": "}"
    }
  ]
}
```

Field names use `snake_case`. Applicability values serialize as lowercase
underscore strings.

---

## Implementation Notes

- `Suggestion` is a value type. It is owned by the `Diagnostic` struct defined
  in the diagnostics-runtime spec.
- Edits within one `Suggestion` must not overlap; callers are responsible for
  ensuring non-overlapping spans. The implementation may assert on this in debug
  builds.
- Display order for multiple suggestions: `is_primary == true` first, then
  `MachineApplicable` before `MaybeApplicable` before `Manual`.
- Recovery actions emitted by the parser are represented as `Suggestion` values
  with `applicability: Applicability::Manual`; they explain what the parser did,
  not what the user should do.
# Suggestions and Fixes

## Objective

Define how diagnostics can carry actionable edits and guidance that tools can present or apply.

This spec covers:

- structured fixes
- alternative fixes
- applicability/confidence levels
- display-only suggestions versus machine-applicable edits
- grouping multiple edits under one logical fix

## Why This Matters

Hints tell the user what to do. Suggestions tell tooling exactly how to represent or apply the fix.

For parser and tokenizer diagnostics, examples include:

- insert a missing delimiter
- replace an invalid token with the expected one
- remove an extra separator
- normalize a regex pattern by prefixing `^`

## Core Model

Likely types:

```rust
pub enum EditKind {
    Insert,
    Replace,
    Delete,
}

pub enum Applicability {
    MachineApplicable,
    MaybeApplicable,
    Manual,
}

pub struct TextEdit {
    pub span: SourceSpan,
    pub replacement: String,
    pub kind: EditKind,
}

pub struct Suggestion {
    pub title: String,
    pub edits: Vec<TextEdit>,
    pub applicability: Applicability,
}
```

## Requirements

1. A diagnostic may have zero, one, or many suggestions.
2. A suggestion may contain multiple coordinated edits.
3. Suggestions must be serializable in JSON output.
4. Renderers must be able to display suggestions without applying them.
5. Applicability must be explicit so editor tooling can decide whether to auto-apply.

## Distinguishing Hints from Suggestions

- hint: human advice, usually prose
- suggestion: structured candidate fix, often tied to spans and text edits

Example:

- hint: "Add a closing `)` to finish the call expression."
- suggestion: insert `)` at byte offset 120

## Alternative Suggestions

Some diagnostics have multiple valid fixes.

Examples:

- remove the trailing comma
- or add another list item

The system must allow multiple suggestions to coexist, rather than forcing a single canonical fix.

## Display Requirements

Terminal and plain renderers should show:

- suggestion title
- replacement preview
- applicability

JSON renderers should preserve full edit structure, including spans and replacements.

## Parser Recovery Use Cases

Parser recovery often suggests what the parser inserted or skipped.

The system should distinguish:

- actual safe fix suggestions
- explanatory recovery actions

Not every recovery action should be offered as a machine-applicable fix.

## Open Questions

1. Should zero-length insertion edits use ordinary spans or a dedicated insertion-point type?
2. Should suggestions support formatted diff previews in addition to plain replacements?
3. Should one diagnostic be able to mark one suggestion as preferred?