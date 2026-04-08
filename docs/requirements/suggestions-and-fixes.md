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