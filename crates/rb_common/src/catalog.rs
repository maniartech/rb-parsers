// ── ErrorCode ─────────────────────────────────────────────────────────────────

/// A stable, namespaced error code (e.g. `ErrorCode("RBP-unexpected-token")`).
/// Always `'static` — zero heap allocation in the common path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ErrorCode(pub &'static str);

impl ErrorCode {
    pub fn as_str(self) -> &'static str { self.0 }
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

// ── ErrorSeverity ─────────────────────────────────────────────────────────────

/// Severity level for diagnostics. Ordered from lowest to highest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorSeverity {
    Info,
    Hint,
    Warning,
    Error,
}

impl ErrorSeverity {
    pub fn is_error(self) -> bool { self == ErrorSeverity::Error }
    pub fn is_warning(self) -> bool { self == ErrorSeverity::Warning }
}

impl std::fmt::Display for ErrorSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorSeverity::Info    => write!(f, "info"),
            ErrorSeverity::Hint    => write!(f, "hint"),
            ErrorSeverity::Warning => write!(f, "warning"),
            ErrorSeverity::Error   => write!(f, "error"),
        }
    }
}

// ── ErrorTemplate ─────────────────────────────────────────────────────────────

/// A static, documentation-only record for one error code.
/// Never heap-allocated at runtime.
#[derive(Debug)]
pub struct ErrorTemplate {
    pub code: ErrorCode,
    pub severity: ErrorSeverity,
    /// Short, stable human-readable name.
    pub title: &'static str,
    /// Message with `{placeholder}` holes. Substitution is the caller's responsibility.
    pub message_template: &'static str,
    /// Default hint strings displayed when no better hint is produced.
    pub default_hints: &'static [&'static str],
    pub docs_slug: &'static str,
    pub deprecation: Option<&'static str>,
}

// ── ErrorCatalog trait ────────────────────────────────────────────────────────

/// A queryable registry of error templates. Object-safe.
pub trait ErrorCatalog: Send + Sync {
    /// Returns the template for the given code, or `None` if not found.
    fn get(&self, code: ErrorCode) -> Option<&ErrorTemplate>;
    /// Returns the namespace prefix for this catalog (e.g. `"RBP"`).
    fn namespace(&self) -> &'static str;
}

// ── StaticErrorCatalog ────────────────────────────────────────────────────────

/// A registry of [`ErrorTemplate`]s under a single namespace (e.g. `"RBP"`).
/// Backed by a static slice — zero allocation.
pub struct StaticErrorCatalog {
    pub namespace: &'static str,
    pub templates: &'static [ErrorTemplate],
}

impl StaticErrorCatalog {
    /// Look up a template by its full code string.
    pub fn lookup(&self, code: &str) -> Option<&ErrorTemplate> {
        self.templates.iter().find(|t| t.code.as_str() == code)
    }
}

impl ErrorCatalog for StaticErrorCatalog {
    fn get(&self, code: ErrorCode) -> Option<&ErrorTemplate> {
        self.lookup(code.as_str())
    }
    fn namespace(&self) -> &'static str { self.namespace }
}

// ── CompositeErrorCatalog ─────────────────────────────────────────────────────

/// Fan-out catalog that queries multiple child catalogs in order.
pub struct CompositeErrorCatalog {
    catalogs: Vec<Box<dyn ErrorCatalog>>,
}

impl CompositeErrorCatalog {
    pub fn new() -> Self {
        CompositeErrorCatalog { catalogs: Vec::new() }
    }

    pub fn with(mut self, catalog: Box<dyn ErrorCatalog>) -> Self {
        self.catalogs.push(catalog);
        self
    }
}

impl Default for CompositeErrorCatalog {
    fn default() -> Self { Self::new() }
}

impl ErrorCatalog for CompositeErrorCatalog {
    fn get(&self, code: ErrorCode) -> Option<&ErrorTemplate> {
        for cat in &self.catalogs {
            if let Some(t) = cat.get(code) {
                return Some(t);
            }
        }
        None
    }
    fn namespace(&self) -> &'static str { "<composite>" }
}
