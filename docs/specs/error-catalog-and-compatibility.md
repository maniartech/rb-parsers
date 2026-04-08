# Spec: Error Catalog and Compatibility

**Status**: Ready for implementation
**Module**: `rb_common::catalog`
**Depends on**: nothing (pure metadata; no span or sink dependency)
**Requirement source**: `docs/requirements/error-catalog-and-compatibility.md`

---

## Decisions Made

| Question | Decision |
|---|---|
| Documentation generation | External tool or build script; the catalog API provides the data, generation lives outside `rb_common` |
| Test stability | Tests should assert on error code and structural fields only, not on message wording |
| Composable catalogs | Catalogs are composable: crates register their own `ErrorCatalog` and the workspace can merge them into a single view at runtime |
| `templates()` return type | Returns `Vec<&ErrorTemplate>` (not `&[ErrorTemplate]`) because filtering requires allocation; `all_templates()` also returns `Vec<&ErrorTemplate>` for uniformity |
| `RBC` namespace | Reserved for `rb_common` framework-level diagnostics. Phase 1 defines zero `RBC-*` codes because `rb_common` itself has no user-facing errors yet. The namespace exists in documentation and is registered in `CompositeErrorCatalog` with an empty template list so tooling can enumerate it. |

---

## Namespace Conventions

Error codes follow the pattern `{NAMESPACE}-{kebab-slug}`:

| Crate | Namespace |
|---|---|
| `rb_common` | `RBC` |
| `rb_tokenizer` | `RBT` |
| `rb_parser` | `RBP` |

- `NAMESPACE` is the uppercase crate prefix.
- `slug` is a lowercase, kebab-case description of what went wrong — chosen to be self-explanatory without consulting docs.
- Severity (`error`, `warning`, `note`) is not encoded in the code string; it lives in `ErrorTemplate::severity`. This keeps codes readable at a glance.

Examples: `RBT-unrecognized-char`, `RBT-pattern-auto-anchored`, `RBP-unexpected-token`, `RBC-invalid-span`.

---

## Module Layout

```
rb_common::catalog
├── ErrorCode
├── ErrorSeverity
├── DeprecationInfo
├── ErrorTemplate
├── ErrorCatalog  (trait)
├── StaticErrorCatalog
└── CompositeErrorCatalog
```

---

## Types

### ErrorCode

A stable, opaque error code. Stored as a `&'static str` so it is zero-copy and
avoids allocation in the hot path.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ErrorCode(pub &'static str);

impl ErrorCode {
    pub fn as_str(self) -> &'static str {
        self.0
    }
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}
```

---

### ErrorSeverity

The diagnostic severity associated with an error template.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorSeverity {
    Note,
    Warning,
    Error,
}

impl std::fmt::Display for ErrorSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorSeverity::Note => write!(f, "note"),
            ErrorSeverity::Warning => write!(f, "warning"),
            ErrorSeverity::Error => write!(f, "error"),
        }
    }
}
```

---

### DeprecationInfo

Optional deprecation metadata on a template that has been superseded.

```rust
#[derive(Debug, Clone)]
pub struct DeprecationInfo {
    /// The replacement code, if a direct successor exists.
    pub replaced_by: Option<ErrorCode>,
    /// A short human-readable note explaining the deprecation.
    pub note: &'static str,
}
```

---

### ErrorTemplate

A static definition of one error kind. All fields use `'static` lifetimes so
template arrays can live in the data segment with zero runtime overhead.

```rust
#[derive(Debug, Clone)]
pub struct ErrorTemplate {
    /// The stable error code. Must be unique within the owning catalog.
    pub code: ErrorCode,
    /// Default severity. May be overridden by runtime policy.
    pub severity: ErrorSeverity,
    /// Short human-readable title (used in documentation headings).
    pub title: &'static str,
    /// Default message template. Supports `{placeholder}` substitutions when
    /// formatted by the diagnostics layer. Plain prose is also acceptable.
    pub message_template: &'static str,
    /// Optional default hints emitted when no user-authored hint is present.
    pub default_hints: &'static [&'static str],
    /// Documentation slug, used for generating stable anchor URLs.
    /// Must be unique within the owning catalog.
    pub docs_slug: &'static str,
    /// Optional deprecation metadata.
    pub deprecation: Option<DeprecationInfo>,
}

impl ErrorTemplate {
    pub fn is_deprecated(&self) -> bool {
        self.deprecation.is_some()
    }
}
```

---

### ErrorCatalog (trait)

The query interface implemented by both static and composite catalogs.

```rust
pub trait ErrorCatalog: Send + Sync {
    /// Returns the namespace prefix for this catalog (e.g. `"RBT"`).
    fn namespace(&self) -> &'static str;

    /// Look up a template by its exact code string.
    fn get(&self, code: ErrorCode) -> Option<&ErrorTemplate>;

    /// Returns all non-deprecated templates in this catalog.
    ///
    /// Returns `Vec<&ErrorTemplate>` rather than `&[ErrorTemplate]` because
    /// filtering deprecated entries at call time requires allocation. Static
    /// catalogs that pre-filter at compile time may intern the filtered slice
    /// and return it via `all_templates()` (which is zero-copy).
    fn templates(&self) -> Vec<&ErrorTemplate>;

    /// Returns all templates in this catalog including deprecated ones.
    /// For `StaticErrorCatalog` this is a zero-copy `&'static [ErrorTemplate]`
    /// slice. The default delegates to `templates()` for composite catalogs.
    fn all_templates(&self) -> Vec<&ErrorTemplate> {
        self.templates()
    }

    /// Returns all templates matching the given severity.
    fn by_severity(&self, severity: ErrorSeverity) -> Vec<&ErrorTemplate> {
        self.all_templates()
            .into_iter()
            .filter(|t| t.severity == severity)
            .collect()
    }
}
```

---

### StaticErrorCatalog

The default implementation backed by a `&'static [ErrorTemplate]` slice.
Intended to be constructed in each crate's module-level `const` or `static`.

```rust
pub struct StaticErrorCatalog {
    pub namespace: &'static str,
    pub templates: &'static [ErrorTemplate],
}

impl ErrorCatalog for StaticErrorCatalog {
    fn namespace(&self) -> &'static str {
        self.namespace
    }

    fn get(&self, code: ErrorCode) -> Option<&ErrorTemplate> {
        self.templates.iter().find(|t| t.code == code)
    }

    /// Returns non-deprecated templates. Because `self.templates` is a
    /// `&'static [ErrorTemplate]` we can filter lazily on each call.
    /// Production catalogs may pre-split deprecated/active entries at
    /// build time to avoid the filter entirely.
    fn templates(&self) -> Vec<&ErrorTemplate> {
        self.templates.iter().filter(|t| !t.is_deprecated()).collect()
    }

    /// Returns all templates including deprecated ones as a zero-copy
    /// iterator over the static slice.
    fn all_templates(&self) -> Vec<&ErrorTemplate> {
        self.templates.iter().collect()
    }
}
```

> **Implementation note**: `templates()` on `StaticErrorCatalog` allocates a
> `Vec` of references on each call because filtering deprecated entries requires
> traversal. Production catalogs that need zero-allocation hot paths should split
> the `&'static [ErrorTemplate]` slice into two parts at build time (active vs.
> deprecated) and return `self.active_templates.iter().collect()` or simply
> expose a `static_active_templates()` method that returns `&'static [ErrorTemplate]`
> directly. `all_templates()` stays zero-copy for use by documentation tooling.

---

### CompositeErrorCatalog

Merges multiple catalogs for workspace-wide queries such as documentation
generation.

```rust
pub struct CompositeErrorCatalog {
    parts: Vec<Box<dyn ErrorCatalog>>,
}

impl CompositeErrorCatalog {
    pub fn new() -> Self {
        CompositeErrorCatalog { parts: Vec::new() }
    }

    pub fn add(mut self, catalog: Box<dyn ErrorCatalog>) -> Self {
        self.parts.push(catalog);
        self
    }

    pub fn all_templates_flat(&self) -> Vec<&ErrorTemplate> {
        self.parts.iter().flat_map(|c| c.all_templates()).collect()
    }

    pub fn get_any(&self, code: ErrorCode) -> Option<&ErrorTemplate> {
        self.parts.iter().find_map(|c| c.get(code))
    }
}
```

---

## Stability Rules

| Field | Stability |
|---|---|
| `code` | Stable once released |
| `severity` | Stable, except to correct a wrong classification |
| `title` | Mostly stable; minor rewording allowed |
| `message_template` | May improve; should avoid unnecessary churn |
| `docs_slug` | Stable once published |
| JSON schema version | Explicit and versioned; see diagnostics-runtime spec |

---

## Validation Checklist

The workspace CI should eventually assert:

- [ ] No duplicate `code` values within a namespace
- [ ] No duplicate `docs_slug` values within a namespace
- [ ] All `default_hints` entries are non-empty strings
- [ ] `message_template` contains no broken `{placeholder}` references
- [ ] Every deprecated template has either a `replaced_by` code or a non-empty `note`

---

## Usage Example

```rust
// In rb_common (reserved; no active codes in Phase 1)
static COMMON_CATALOG: StaticErrorCatalog = StaticErrorCatalog {
    namespace: "RBC",
    templates: &[], // No user-facing diagnostics in Phase 1.
};

// In rb_tokenizer
static TOKENIZER_CATALOG: StaticErrorCatalog = StaticErrorCatalog {
    namespace: "RBT",
    templates: &[
        ErrorTemplate {
            code: ErrorCode("RBT-unrecognized-char"),
            severity: ErrorSeverity::Error,
            title: "Unrecognized character",
            message_template: "unrecognized character `{char}` at this position",
            default_hints: &["If this character is part of a valid token, add a scanner for it before the fallback scanner."],
            docs_slug: "rbt-unrecognized-char",
            deprecation: None,
        },
    ],
};
```

### RBC Namespace

`rb_common` owns the `RBC` namespace. In Phase 1, `rb_common` emits no
user-facing diagnostics of its own — it is purely infrastructure. The `RBC`
namespace is therefore **registered but empty**.

Future additions (e.g. `RBC-invalid-span`, `RBC-catalog-mismatch`) must go
through the standard `ErrorTemplate` registration path and will be added in a
later phase alongside concrete `rb_common` validation logic. The namespace must
not be claimed by any other crate.

---

## Implementation Notes

- All string slices in `ErrorTemplate` are `'static` so the `StaticErrorCatalog`
  can be a `static` item with zero heap allocation.
- `CompositeErrorCatalog` is the right type for documentation tooling and
  workspace-level error reference generation. It is not meant to be on the hot path.
- Message template substitution (`{placeholder}` → concrete values) is performed
  by the diagnostics-runtime layer, not by the catalog itself. The catalog stores
  only the raw template.
# Error Catalog and Compatibility

## Objective

Define how error templates are organized, versioned, validated, and documented across the workspace.

This spec makes the template-backed error system stable enough for tests, documentation, and tooling integrations.

## Namespacing

Error codes should be namespaced by subsystem.

Initial direction:

- `RBC` for common/framework-level codes
- `RBT` for tokenizer
- `RBP` for parser

Future extracted crates should define their own namespace only when they become real public subsystems.

Examples:

- `RBT-E001`
- `RBT-W010`
- `RBP-E101`

## Catalog Requirements

Each catalog must support:

1. unique codes within its namespace
2. stable human-readable titles
3. stable documentation slugs
4. lookup by code
5. listing for documentation generation

## Compatibility Rules

The system should define what is stable and what may change.

Recommended stability policy:

- error code: stable once released
- severity: stable unless the old classification was wrong
- title: mostly stable
- default message wording: may improve, but should avoid unnecessary churn
- docs slug: stable once published
- JSON schema version: explicit and versioned

## Deprecation

Sometimes an error template will need replacement.

The catalog should support deprecation metadata:

```rust
pub struct DeprecationInfo {
    pub replaced_by: Option<ErrorCode>,
    pub note: &'static str,
}
```

This makes it possible to keep documentation and migration guidance coherent.

## Validation

The workspace should eventually support automated checks for:

- duplicate codes
- duplicate docs slugs
- missing hints where policy requires them
- missing documentation entries
- broken template placeholders

## JSON Compatibility

Because diagnostics may be consumed by tools, the JSON output shape should have a schema version.

Likely top-level field:

```json
{
  "schema_version": 1,
  "diagnostics": []
}
```

Schema changes should follow explicit compatibility rules.

## Documentation Generation

Documentation generation should be able to produce:

- error reference pages
- grouped catalogs by subsystem
- hint and suggestion examples
- deprecation notices

This can live either inside `rb_common` or in a companion tool, but the catalog format must support it directly.

## Open Questions

1. Should documentation generation be a library feature, build script, or external tool?
2. How much message wording stability should tests depend on versus asserting only error code and structural fields?
3. Should crate-local catalogs be composable into a single workspace-wide catalog at runtime?