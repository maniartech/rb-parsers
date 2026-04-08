# Spec: Parser Error Catalog (RBP namespace)

**Status**: Ready for implementation
**Module**: `rb_parser::catalog`
**Depends on**: `rb_common::catalog` (`ErrorTemplate`, `StaticErrorCatalog`, `ErrorCode`, `ErrorSeverity`)
**Requirement source**: `docs/requirements/error-catalog-and-compatibility.md`,
`docs/specs/error-catalog-and-compatibility.md`

---

## Decisions Made

| Question | Decision |
|---|---|
| Namespace | `RBP` (rb_parser). All parser error codes use slug-based `RBP-{kebab-slug}` format. |
| Severity encoding | Not encoded in the code string. Lives in `ErrorTemplate::severity`. |
| Grammar-compile-time errors | `GrammarError` variants (`LeftRecursion`, `UnreachableBranch`, etc.) are Rust enum variants returned from `Grammar::compile()`. They do NOT emit `RBP-*` diagnostics because they occur before parsing begins and are developer-only, not end-user-facing. |
| Grammar-compile-time codes | Reserved as `RBP-left-recursion`, `RBP-unreachable-branch`, `RBP-conflicting-guards` so future tooling (e.g. grammar linters) can emit them as diagnostics. Not emitted by the runtime in Phase 1. |
| Catalog registration | A single `static RBP_CATALOG: StaticErrorCatalog` in `rb_parser::catalog`. |
| Test stability | Tests must assert on `code` and `severity` only; not on `message_template` wording. |

---

## Namespace

```rust
pub const RBP_NAMESPACE: &str = "RBP";
```

---

## Error Code Constants

Each constant is an `ErrorCode` wrapping the canonical slug string. Use these
constants everywhere a parser diagnostic is constructed, rather than bare
string literals.

```rust
use rb_common::catalog::ErrorCode;

// ── Runtime parse errors ─────────────────────────────────────────────────────

/// An unexpected token was encountered where a specific token was required.
/// Emitted on `CommittedFailure` caused by a token mismatch.
pub const RBP_UNEXPECTED_TOKEN:    ErrorCode = ErrorCode("RBP-unexpected-token");

/// A required token was absent at the expected position.
/// Emitted when the engine inserts a synthetic token for recovery.
pub const RBP_MISSING_TOKEN:       ErrorCode = ErrorCode("RBP-missing-token");

/// An opening delimiter was not matched by a closing delimiter before EOF or
/// a sibling closing delimiter was found.
pub const RBP_UNMATCHED_DELIMITER: ErrorCode = ErrorCode("RBP-unmatched-delimiter");

/// The engine exhausted its configured `RecoveryConfig::max_recovery_steps`
/// budget before the parse could complete.
pub const RBP_RECOVERY_LIMIT:      ErrorCode = ErrorCode("RBP-recovery-limit");

// ── Grammar compile-time reservations (not emitted at runtime in Phase 1) ────

/// The grammar contains a direct or indirect left-recursive cycle.
/// Reserved for future grammar tooling; `Grammar::compile()` returns
/// `GrammarError::LeftRecursion` as a Rust error in Phase 1.
pub const RBP_LEFT_RECURSION:      ErrorCode = ErrorCode("RBP-left-recursion");

/// A branch in `one_of!` can never be reached.
/// Reserved for future grammar linting.
pub const RBP_UNREACHABLE_BRANCH:  ErrorCode = ErrorCode("RBP-unreachable-branch");

/// Two or more profile guards on alternatives in `one_of!` are simultaneously
/// active for a resolved profile at compile time.
/// Reserved for future grammar linting.
pub const RBP_CONFLICTING_GUARDS:  ErrorCode = ErrorCode("RBP-conflicting-guards");
```

---

## ErrorTemplate Definitions

```rust
use rb_common::catalog::{ErrorTemplate, ErrorSeverity, DeprecationInfo};

pub static RBP_TEMPLATES: &[ErrorTemplate] = &[
    // ── Runtime errors ─────────────────────────────────────────────────────

    ErrorTemplate {
        code:             ErrorCode("RBP-unexpected-token"),
        severity:         ErrorSeverity::Error,
        title:            "Unexpected token",
        message_template: "expected {expected} but found `{found}`",
        default_hints:    &[],
        docs_slug:        "rbp-unexpected-token",
        deprecation:      None,
    },

    ErrorTemplate {
        code:             ErrorCode("RBP-missing-token"),
        severity:         ErrorSeverity::Error,
        title:            "Missing token",
        message_template: "expected {expected} before `{found}`",
        default_hints:    &[
            "The parser inserted a synthetic placeholder so it could continue. \
             Fix the source to include the missing token.",
        ],
        docs_slug:        "rbp-missing-token",
        deprecation:      None,
    },

    ErrorTemplate {
        code:             ErrorCode("RBP-unmatched-delimiter"),
        severity:         ErrorSeverity::Error,
        title:            "Unmatched delimiter",
        message_template: "opening `{open}` was not closed before `{found}`",
        default_hints:    &[
            "Check that every opening delimiter has a matching closing delimiter.",
        ],
        docs_slug:        "rbp-unmatched-delimiter",
        deprecation:      None,
    },

    ErrorTemplate {
        code:             ErrorCode("RBP-recovery-limit"),
        severity:         ErrorSeverity::Warning,
        title:            "Error recovery limit reached",
        message_template: "the parser stopped recovering after {limit} errors; \
                           remaining input was skipped",
        default_hints:    &[
            "Fix earlier errors first — subsequent diagnostics may be cascading errors.",
        ],
        docs_slug:        "rbp-recovery-limit",
        deprecation:      None,
    },

    // ── Grammar compile-time reservations (emitted by future tooling) ──────

    ErrorTemplate {
        code:             ErrorCode("RBP-left-recursion"),
        severity:         ErrorSeverity::Error,
        title:            "Left-recursive grammar rule",
        message_template: "grammar rule `{rule}` is left-recursive through: {cycle}",
        default_hints:    &[
            "PEG parsers do not support left recursion. \
             Rewrite the rule using `repeat0`, `repeat1`, or `list` to express \
             the same language without a left-recursive cycle.",
        ],
        docs_slug:        "rbp-left-recursion",
        deprecation:      None,
    },

    ErrorTemplate {
        code:             ErrorCode("RBP-unreachable-branch"),
        severity:         ErrorSeverity::Warning,
        title:            "Unreachable grammar branch",
        message_template: "branch {branch} of rule `{rule}` can never be reached",
        default_hints:    &[
            "An earlier branch in `one_of!` always succeeds or always commits \
             before this branch is tried. Remove the unreachable branch or \
             reorder the alternatives.",
        ],
        docs_slug:        "rbp-unreachable-branch",
        deprecation:      None,
    },

    ErrorTemplate {
        code:             ErrorCode("RBP-conflicting-guards"),
        severity:         ErrorSeverity::Error,
        title:            "Conflicting profile guards",
        message_template: "two or more branches of `one_of!` in rule `{rule}` \
                           are simultaneously active for the current profile",
        default_hints:    &[
            "Ensure that the `.enabled_if(...)` guards on alternatives in \
             `one_of!` are mutually exclusive.",
        ],
        docs_slug:        "rbp-conflicting-guards",
        deprecation:      None,
    },
];
```

---

## Catalog Registration

```rust
use rb_common::catalog::StaticErrorCatalog;

pub static RBP_CATALOG: StaticErrorCatalog = StaticErrorCatalog {
    namespace: "RBP",
    templates: RBP_TEMPLATES,
};
```

---

## Message Template Placeholders

Each template uses `{placeholder}` syntax. Substitution is performed by the
diagnostics layer (`DiagnosticBuilder::from_template`) before the diagnostic
is emitted.

| Code | Placeholder | Meaning |
|---|---|---|
| `RBP-unexpected-token` | `{expected}` | What the parser expected (e.g. `":"`) |
| `RBP-unexpected-token` | `{found}` | What token was actually seen |
| `RBP-missing-token` | `{expected}` | What token is missing |
| `RBP-missing-token` | `{found}` | The token that appeared next |
| `RBP-unmatched-delimiter` | `{open}` | The opening delimiter (e.g. `"{"`) |
| `RBP-unmatched-delimiter` | `{found}` | The token that closed the wrong delimiter (or `"<EOF>"`) |
| `RBP-recovery-limit` | `{limit}` | The `max_recovery_steps` value that was reached |
| `RBP-left-recursion` | `{rule}` | The name of the entry rule in the cycle |
| `RBP-left-recursion` | `{cycle}` | Comma-separated rule names forming the cycle |
| `RBP-unreachable-branch` | `{rule}` | Enclosing rule name |
| `RBP-unreachable-branch` | `{branch}` | 0-based branch index in `one_of!` |
| `RBP-conflicting-guards` | `{rule}` | Enclosing rule name |

---

## Stability Rules

| Field | Stability |
|---|---|
| `code` | Stable once released |
| `severity` | Stable, except to correct a misclassification |
| `docs_slug` | Stable once published |
| `message_template` | May improve; avoid unnecessary churn |
| Placeholder names | Stable once released; adding new placeholders is non-breaking |

---

## Validation Rules (CI)

The workspace CI should assert:

- [ ] All `RBP_TEMPLATES` entries have unique `code` values within the `RBP` namespace.
- [ ] All `RBP_TEMPLATES` entries have unique `docs_slug` values.
- [ ] All `default_hints` entries are non-empty strings.
- [ ] Every placeholder referenced in `message_template` is documented in this spec.

---

## Usage Example

```rust
use rb_parser::catalog::RBP_CATALOG;
use rb_parser::catalog::RBP_UNEXPECTED_TOKEN;
use rb_common::catalog::ErrorCatalog;
use rb_common::diagnostics::DiagnosticBuilder;

// Emit an "unexpected token" diagnostic
let diag = DiagnosticBuilder::from_template(
    &RBP_CATALOG,
    RBP_UNEXPECTED_TOKEN,
    format!("expected `:` but found `}}`"),
)
.primary(location)
.hint("Add a `:` between the key and value.")
.build();

ctx.emit(diag);
```
