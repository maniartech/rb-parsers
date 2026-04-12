use rb_common::catalog::{ErrorCode, ErrorSeverity, ErrorTemplate, StaticErrorCatalog};

// ── Namespace ──────────────────────────────────────────────────────────────────

/// Error code namespace prefix for the `rb_parser` crate.
pub const RBP_NAMESPACE: &str = "RBP";

// ── Error code constants ───────────────────────────────────────────────────

/// Emitted when the parser encounters a token it did not expect.
pub const RBP_UNEXPECTED_TOKEN:    ErrorCode = ErrorCode("RBP-unexpected-token");
/// Emitted when a required token is absent from the input.
pub const RBP_MISSING_TOKEN:       ErrorCode = ErrorCode("RBP-missing-token");
/// Emitted when an opening delimiter is not matched by a closing one.
pub const RBP_UNMATCHED_DELIMITER: ErrorCode = ErrorCode("RBP-unmatched-delimiter");
/// Emitted when the error-recovery budget is exhausted.
pub const RBP_RECOVERY_LIMIT:      ErrorCode = ErrorCode("RBP-recovery-limit");
/// Emitted when a left-recursive grammar rule is detected at compile time.
pub const RBP_LEFT_RECURSION:      ErrorCode = ErrorCode("RBP-left-recursion");
/// Emitted when a grammar branch can never be reached.
pub const RBP_UNREACHABLE_BRANCH:  ErrorCode = ErrorCode("RBP-unreachable-branch");
/// Emitted when two or more profile-guarded branches are simultaneously active.
pub const RBP_CONFLICTING_GUARDS:  ErrorCode = ErrorCode("RBP-conflicting-guards");

// ── Template definitions ───────────────────────────────────────────────────────

/// All `rb_parser` error templates, keyed by their [`ErrorCode`].
pub static RBP_TEMPLATES: &[ErrorTemplate] = &[
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
    ErrorTemplate {
        code:             ErrorCode("RBP-left-recursion"),
        severity:         ErrorSeverity::Error,
        title:            "Left-recursive grammar rule",
        message_template: "grammar rule `{rule}` is left-recursive through: {cycle}",
        default_hints:    &[
            "PEG parsers do not support left recursion. \
             Rewrite the rule using `repeat0`, `repeat1`, or `list`.",
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
            "An earlier branch always succeeds or commits before this branch is tried.",
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
            "Ensure that `.enabled_if(...)` guards on alternatives are mutually exclusive.",
        ],
        docs_slug:        "rbp-conflicting-guards",
        deprecation:      None,
    },
];

// ── Catalog ────────────────────────────────────────────────────────────────────

/// The complete static error catalog for `rb_parser`.
pub static RBP_CATALOG: StaticErrorCatalog = StaticErrorCatalog {
    namespace: "RBP",
    templates: RBP_TEMPLATES,
};
