# Spec: rb_tokenizer — Error Catalog (RBT namespace)

**Status**: Ready for implementation
**Crate**: `rb_tokenizer`
**Module**: `rb_tokenizer::catalog`
**Depends on**: `rb_common::catalog` (spec: `docs/specs/error-catalog-and-compatibility.md`)
**Requirement source**: `docs/requirements/error-catalog-and-compatibility.md`, `docs/requirements/error-system.md`

---

## Namespace

All codes in this catalog use the `RBT` namespace.

Format: `RBT-{kebab-slug}`

The slug is a lowercase, kebab-case description of what went wrong.
Severity is **not** encoded in the code string; it is carried by `ErrorTemplate::severity`.
This makes codes self-explanatory at a glance — for developers, tooling, and error messages alike.

---

## Module Layout

```
rb_tokenizer::catalog
├── RBT_CATALOG                 (static StaticErrorCatalog)
└── codes                       (public ErrorCode constants)
    ├── RBT_UNRECOGNIZED_CHAR   "RBT-unrecognized-char"
    ├── RBT_UNMATCHED_BLOCK     "RBT-unmatched-block"
    ├── RBT_INVALID_PATTERN     "RBT-invalid-pattern"
    ├── RBT_UNTERMINATED_BLOCK  "RBT-unterminated-block"
    ├── RBT_PATTERN_AUTO_ANCHORED "RBT-pattern-auto-anchored"
    └── RBT_ERROR_LIMIT_REACHED "RBT-error-limit-reached"
```

---

## Error Code Definitions

### RBT-unrecognized-char — Unrecognized character

**Severity**: Error
**When**: The tokenizer encounters a character at the current position that no
registered scanner can match, and the result is a hard failure (not recovered).
Handles all Unicode scalar values; `{char}` in the message is the full Unicode character.

```rust
ErrorTemplate {
    code: ErrorCode("RBT-unrecognized-char"),
    severity: ErrorSeverity::Error,
    title: "Unrecognized character",
    message_template: "unrecognized character `{char}` at this position",
    default_hints: &[
        "If this character is part of a valid token, register a scanner for it \
         before the fallback identifier scanner.",
        "For Unicode identifiers, use `\\p{ID_Start}` and `\\p{ID_Continue}` \
         in your regex scanner pattern.",
    ],
    docs_slug: "RBT-unrecognized-char",
    deprecation: None,
}
```

---

### RBT-unmatched-block — Unmatched block delimiter

**Severity**: Error
**When**: A block scanner found an opening delimiter but reached end-of-input or
a mismatched closing delimiter before the block was closed.

```rust
ErrorTemplate {
    code: ErrorCode("RBT-unmatched-block"),
    severity: ErrorSeverity::Error,
    title: "Unmatched block delimiter",
    message_template: "block opened with `{start}` was never closed with `{end}`",
    default_hints: &[
        "Add the matching closing delimiter `{end}` to complete the block.",
    ],
    docs_slug: "RBT-unmatched-block",
    deprecation: None,
}
```

---

### RBT-invalid-pattern — Invalid regex pattern

**Severity**: Error
**When**: A regex pattern passed to `add_regex_scanner` fails to compile.
This error is produced at scanner registration time, not during tokenization.

```rust
ErrorTemplate {
    code: ErrorCode("RBT-invalid-pattern"),
    severity: ErrorSeverity::Error,
    title: "Invalid regex pattern",
    message_template: "regex pattern `{pattern}` is invalid: {reason}",
    default_hints: &[
        "Check the pattern for unbalanced brackets, unsupported syntax, or \
         missing escape characters.",
        "To match Unicode scripts, use `\\p{Script=Latin}`, `\\p{Script=Arabic}`, etc.",
    ],
    docs_slug: "RBT-invalid-pattern",
    deprecation: None,
}
```

---

### RBT-unterminated-block — Unterminated block content

**Severity**: Error
**When**: A block scanner matched an opening delimiter and consumed content up to
end-of-input without finding the closing delimiter, distinct from an outright
mismatch. (Fired when `raw_mode` or multiline content runs past EOF.)

```rust
ErrorTemplate {
    code: ErrorCode("RBT-unterminated-block"),
    severity: ErrorSeverity::Error,
    title: "Unterminated block content",
    message_template: "content opened with `{start}` was not terminated before end of input",
    default_hints: &[
        "Ensure the closing delimiter `{end}` appears before the end of the source.",
    ],
    docs_slug: "RBT-unterminated-block",
    deprecation: None,
}
```

---

### RBT-pattern-auto-anchored — Regex pattern auto-prefixed with `^`

**Severity**: Warning
**When**: A regex pattern was passed without a leading `^` anchor and the
tokenizer automatically prefixed it. The pattern still works but the intent
was not explicit.

```rust
ErrorTemplate {
    code: ErrorCode("RBT-pattern-auto-anchored"),
    severity: ErrorSeverity::Warning,
    title: "Regex pattern auto-prefixed with `^`",
    message_template: "pattern `{pattern}` did not start with `^`; `^` was added automatically",
    default_hints: &[
        "Add `^` explicitly to the start of the pattern to make the anchoring intent clear.",
    ],
    docs_slug: "RBT-pattern-auto-anchored",
    deprecation: None,
}
```

---

### RBT-error-limit-reached — Error tolerance limit reached

**Severity**: Warning
**When**: The tokenizer has emitted as many errors as `error_tolerance_limit`
allows and is stopping further recovery attempts.

```rust
ErrorTemplate {
    code: ErrorCode("RBT-error-limit-reached"),
    severity: ErrorSeverity::Warning,
    title: "Error tolerance limit reached",
    message_template: "tokenization stopped after {count} errors; remaining input was not tokenized",
    default_hints: &[
        "Increase `error_tolerance_limit` in `TokenizerConfig` to collect more diagnostics.",
        "Fix the earlier errors to allow tokenization to continue.",
    ],
    docs_slug: "RBT-error-limit-reached",
    deprecation: None,
}
```

---

## Catalog Declaration

```rust
// In rb_tokenizer::catalog (new file: crates/rb_tokenizer/src/catalog.rs)

use rb_common::catalog::{ErrorCode, ErrorSeverity, ErrorTemplate, StaticErrorCatalog};

pub mod codes {
    use rb_common::catalog::ErrorCode;
    pub const RBT_UNRECOGNIZED_CHAR:      ErrorCode = ErrorCode("RBT-unrecognized-char");
    pub const RBT_UNMATCHED_BLOCK:        ErrorCode = ErrorCode("RBT-unmatched-block");
    pub const RBT_INVALID_PATTERN:        ErrorCode = ErrorCode("RBT-invalid-pattern");
    pub const RBT_UNTERMINATED_BLOCK:     ErrorCode = ErrorCode("RBT-unterminated-block");
    pub const RBT_PATTERN_AUTO_ANCHORED:  ErrorCode = ErrorCode("RBT-pattern-auto-anchored");
    pub const RBT_ERROR_LIMIT_REACHED:    ErrorCode = ErrorCode("RBT-error-limit-reached");
}

static TEMPLATES: &[ErrorTemplate] = &[
    ErrorTemplate {
        code: codes::RBT_UNRECOGNIZED_CHAR,
        severity: ErrorSeverity::Error,
        title: "Unrecognized character",
        message_template: "unrecognized character `{char}` at this position",
        default_hints: &[
            "If this character is part of a valid token, register a scanner for it \
             before the fallback identifier scanner.",
            "For Unicode identifiers, use `\\p{ID_Start}` and `\\p{ID_Continue}` \
             in your regex scanner pattern.",
        ],
        docs_slug: "RBT-unrecognized-char",
        deprecation: None,
    },
    ErrorTemplate {
        code: codes::RBT_UNMATCHED_BLOCK,
        severity: ErrorSeverity::Error,
        title: "Unmatched block delimiter",
        message_template: "block opened with `{start}` was never closed with `{end}`",
        default_hints: &[
            "Add the matching closing delimiter `{end}` to complete the block.",
        ],
        docs_slug: "RBT-unmatched-block",
        deprecation: None,
    },
    ErrorTemplate {
        code: codes::RBT_INVALID_PATTERN,
        severity: ErrorSeverity::Error,
        title: "Invalid regex pattern",
        message_template: "regex pattern `{pattern}` is invalid: {reason}",
        default_hints: &[
            "Check the pattern for unbalanced brackets, unsupported syntax, or \
             missing escape characters.",
            "To match Unicode scripts, use `\\p{Script=Latin}`, `\\p{Script=Arabic}`, etc.",
        ],
        docs_slug: "RBT-invalid-pattern",
        deprecation: None,
    },
    ErrorTemplate {
        code: codes::RBT_UNTERMINATED_BLOCK,
        severity: ErrorSeverity::Error,
        title: "Unterminated block content",
        message_template: "content opened with `{start}` was not terminated before end of input",
        default_hints: &[
            "Ensure the closing delimiter `{end}` appears before the end of the source.",
        ],
        docs_slug: "RBT-unterminated-block",
        deprecation: None,
    },
    ErrorTemplate {
        code: codes::RBT_PATTERN_AUTO_ANCHORED,
        severity: ErrorSeverity::Warning,
        title: "Regex pattern auto-prefixed with `^`",
        message_template: "pattern `{pattern}` did not start with `^`; `^` was added automatically",
        default_hints: &[
            "Add `^` explicitly to the start of the pattern to make the anchoring intent clear.",
        ],
        docs_slug: "RBT-pattern-auto-anchored",
        deprecation: None,
    },
    ErrorTemplate {
        code: codes::RBT_ERROR_LIMIT_REACHED,
        severity: ErrorSeverity::Warning,
        title: "Error tolerance limit reached",
        message_template: "tokenization stopped after {count} errors; remaining input was not tokenized",
        default_hints: &[
            "Increase `error_tolerance_limit` in `TokenizerConfig` to collect more diagnostics.",
            "Fix the earlier errors to allow tokenization to continue.",
        ],
        docs_slug: "RBT-error-limit-reached",
        deprecation: None,
    },
];

pub static RBT_CATALOG: StaticErrorCatalog = StaticErrorCatalog {
    namespace: "RBT",
    templates: TEMPLATES,
};
```

---

## Files Changed

| File | Change |
|---|---|
| `crates/rb_tokenizer/src/catalog.rs` | New file — defines `TEMPLATES`, `RBT_CATALOG`, `codes` module |
| `crates/rb_tokenizer/src/lib.rs` | Add `pub mod catalog;` |
| `crates/rb_tokenizer/src/tokenizers/tokenizer.rs` | Import `RBT_CATALOG` and `codes::*` for diagnostic construction |
| `crates/rb_tokenizer/src/scanners/block_scanner.rs` | Use `codes::RBT_UNMATCHED_BLOCK` / `RBT_UNTERMINATED_BLOCK` instead of bare strings |
