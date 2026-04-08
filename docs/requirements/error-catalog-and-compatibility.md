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