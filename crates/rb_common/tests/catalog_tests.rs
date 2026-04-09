use rb_common::catalog::{
    CompositeErrorCatalog, ErrorCatalog, ErrorCode, ErrorSeverity, ErrorTemplate,
    StaticErrorCatalog,
};

// ── Test catalog fixture ──────────────────────────────────────────────────────

static TEST_TEMPLATES: &[ErrorTemplate] = &[
    ErrorTemplate {
        code: ErrorCode("TST-001"),
        severity: ErrorSeverity::Error,
        title: "test error",
        message_template: "unexpected {token}",
        default_hints: &["try removing this token"],
        docs_slug: "tst-001",
        deprecation: None,
    },
    ErrorTemplate {
        code: ErrorCode("TST-002"),
        severity: ErrorSeverity::Warning,
        title: "test warning",
        message_template: "ambiguous {expr}",
        default_hints: &[],
        docs_slug: "tst-002",
        deprecation: Some("use TST-003 instead"),
    },
];

fn test_catalog() -> StaticErrorCatalog {
    StaticErrorCatalog { namespace: "TST", templates: TEST_TEMPLATES }
}

// ── ErrorCode ─────────────────────────────────────────────────────────────────

#[test]
fn error_code_as_str_round_trips() {
    let code = ErrorCode("RBP-unexpected-token");
    assert_eq!(code.as_str(), "RBP-unexpected-token");
    assert_eq!(code.to_string(), "RBP-unexpected-token");
}

#[test]
fn error_code_equality() {
    assert_eq!(ErrorCode("A"), ErrorCode("A"));
    assert_ne!(ErrorCode("A"), ErrorCode("B"));
}

// ── ErrorSeverity ─────────────────────────────────────────────────────────────

#[test]
fn severity_ordering() {
    use ErrorSeverity::*;
    assert!(Info < Hint);
    assert!(Hint < Warning);
    assert!(Warning < Error);
}

#[test]
fn severity_is_error_and_is_warning() {
    assert!(ErrorSeverity::Error.is_error());
    assert!(!ErrorSeverity::Warning.is_error());
    assert!(ErrorSeverity::Warning.is_warning());
    assert!(!ErrorSeverity::Error.is_warning());
}

#[test]
fn severity_display() {
    assert_eq!(ErrorSeverity::Error.to_string(), "error");
    assert_eq!(ErrorSeverity::Warning.to_string(), "warning");
    assert_eq!(ErrorSeverity::Info.to_string(), "info");
    assert_eq!(ErrorSeverity::Hint.to_string(), "hint");
}

// ── StaticErrorCatalog ────────────────────────────────────────────────────────

#[test]
fn static_catalog_get_known_code() {
    let cat = test_catalog();
    let tmpl = cat.get(ErrorCode("TST-001")).unwrap();
    assert_eq!(tmpl.title, "test error");
    assert_eq!(tmpl.severity, ErrorSeverity::Error);
}

#[test]
fn static_catalog_get_unknown_code_returns_none() {
    let cat = test_catalog();
    assert!(cat.get(ErrorCode("TST-999")).is_none());
}

#[test]
fn static_catalog_namespace() {
    assert_eq!(test_catalog().namespace(), "TST");
}

#[test]
fn static_catalog_template_has_deprecation() {
    let cat = test_catalog();
    let tmpl = cat.get(ErrorCode("TST-002")).unwrap();
    assert!(tmpl.deprecation.is_some());
}

#[test]
fn static_catalog_template_has_default_hints() {
    let cat = test_catalog();
    let tmpl = cat.get(ErrorCode("TST-001")).unwrap();
    assert_eq!(tmpl.default_hints, &["try removing this token"]);
}

// ── CompositeErrorCatalog ─────────────────────────────────────────────────────

static SECOND_TEMPLATES: &[ErrorTemplate] = &[ErrorTemplate {
    code: ErrorCode("SND-001"),
    severity: ErrorSeverity::Info,
    title: "second catalog code",
    message_template: "info {x}",
    default_hints: &[],
    docs_slug: "snd-001",
    deprecation: None,
}];

#[test]
fn composite_catalog_finds_code_in_first_catalog() {
    let composite = CompositeErrorCatalog::new()
        .with(Box::new(test_catalog()))
        .with(Box::new(StaticErrorCatalog { namespace: "SND", templates: SECOND_TEMPLATES }));

    assert!(composite.get(ErrorCode("TST-001")).is_some());
}

#[test]
fn composite_catalog_finds_code_in_second_catalog() {
    let composite = CompositeErrorCatalog::new()
        .with(Box::new(test_catalog()))
        .with(Box::new(StaticErrorCatalog { namespace: "SND", templates: SECOND_TEMPLATES }));

    let tmpl = composite.get(ErrorCode("SND-001")).unwrap();
    assert_eq!(tmpl.severity, ErrorSeverity::Info);
}

#[test]
fn composite_catalog_returns_none_when_not_found() {
    let composite = CompositeErrorCatalog::new().with(Box::new(test_catalog()));
    assert!(composite.get(ErrorCode("ABSENT-001")).is_none());
}

#[test]
fn composite_catalog_empty_returns_none() {
    let composite = CompositeErrorCatalog::new();
    assert!(composite.get(ErrorCode("TST-001")).is_none());
}
