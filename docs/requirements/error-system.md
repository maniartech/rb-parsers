# Error System Draft

## Objective

Define a shared, elegant, code-based error and diagnostics system for the Rust Parsers workspace.

This document is the umbrella spec. The detailed companion specs are:

1. `source-spans-and-labels.md`
2. `recovery-and-error-boundaries.md`
3. `suggestions-and-fixes.md`
4. `diagnostics-runtime.md`
5. `error-catalog-and-compatibility.md`

Supporting platform specs:

1. `renderers-and-output.md`
2. `environment-detection.md`

Hinting-specific guidance:

1. `automatic-hinting.md`

The system should support:

- structured error codes
- rich human guidance with notes and help text
- optional hints associated with specific errors
- high-quality automatic fallback hints when user-defined hints are absent
- hierarchical context ranges and ancestor scope information for diagnostics
- continue-on-error workflows with explicit recovery boundaries and resume rules
- machine-readable JSON-style output
- terminal rendering with color when appropriate
- plain rendering for logs and non-color environments
- library-controlled emission through configurable sinks or hooks
- predefined error templates that can be listed, reused, and documented centrally

## Intended Users

1. Library developers building tokenizers, parsers, and related traversal layers
2. Language authors defining language-specific diagnostics
3. Tooling consumers such as CLIs, editors, and CI systems

## High-Level Requirements

1. Diagnostics and errors must be usable across `rb_tokenizer` and `rb_parser`.
2. The design must support both fatal errors and non-fatal warnings.
3. Output formatting must be decoupled from diagnostic creation.
4. The system must support stable error codes for documentation and testing.
5. The library must not hardcode stderr output inside low-level components.
6. Developers must be able to attach optional hints to an error when guidance is useful.
7. When explicit hints are absent, the framework should be able to synthesize high-quality contextual fallback hints.
8. The automatic hinting system must prefer silence over generic or low-confidence garbage.
9. Error definitions should be template-backed so all known errors can be listed in one place.
10. The system should make it possible to generate user-facing documentation from the same template catalog used by the code.
11. Diagnostics must be able to carry enough structural context for renderers to show the immediate failure region, the owning region, and relevant ancestors.
12. Tokenizer and parser must support configurable continue-on-error behavior with explicit recovery boundaries rather than blind best-effort continuation.

## Terminology

- `ErrorTemplate`: a predefined error definition with a stable code, title, default message template, default hints, and documentation metadata
- `Diagnostic`: a concrete emitted instance of a warning or error
- `Hint`: short actionable guidance that helps the user understand what to do next
- `Suggestion`: a structured replacement or edit that tooling may apply or display
- `ErrorCatalog`: a registry of all predefined templates for a crate, language, or subsystem

## Hints

Hints should be first-class citizens in the system.

They are distinct from other diagnostic fields:

- `message`: describes what went wrong
- `notes`: gives additional factual context
- `help` or `hints`: tells the user what to try next
- `suggestions`: provides structured edits or replacements

Hints must be optional because not every error benefits from guidance. However, the system should make them easy to attach so language authors can provide better UX where it matters.

When user-authored hints are not present, the framework should be able to generate fallback hints from structured context such as:

- the error code or template
- spans and labels
- expected-versus-found token information
- tokenizer or parser recovery actions
- subsystem-specific hint providers

Examples of good hints:

- "Add a closing `}` before the end of the block."
- "Prefix the regex with `^` explicitly to make the intention clear."
- "If this token should be allowed in your language, register a scanner for it before the fallback identifier scanner."

Examples of unacceptable fallback hints:

- "Check your syntax."
- "There may be an issue near this code."
- repeating the message text without adding concrete guidance

## Hint Sources and Precedence

Hints should come from a clear precedence order.

Recommended order:

1. explicit user-authored diagnostic hints
2. template default hints
3. subsystem-specific automatic hint providers
4. framework-level generic providers only when they can still produce concrete, contextual advice
5. no hint at all when quality is too low

The framework should never force a generic hint just to ensure every diagnostic has one.

## Template-Backed Error Definitions

Errors should not be invented ad hoc throughout the codebase.

Instead, most reusable errors should come from predefined templates so they can be:

- discovered in one place
- reused consistently across the codebase
- tested against stable codes and messages
- rendered consistently in terminal or JSON output
- documented automatically

A likely shape is:

```rust
pub struct ErrorTemplate {
		pub code: ErrorCode,
		pub severity: Severity,
		pub title: &'static str,
		pub message_template: &'static str,
		pub default_hints: &'static [&'static str],
		pub docs_slug: &'static str,
}
```

Concrete errors would then be emitted by binding runtime values into a template rather than constructing every field manually each time.

Example:

```rust
let diagnostic = error_templates::REGEX_PATTERN_NORMALIZED
		.instantiate()
		.with_message_arg("pattern", pattern)
		.with_hint("Add '^' explicitly to make the anchoring intent clear.");
```

## Error Catalog and Documentation

Each crate or subsystem should be able to define an error catalog.

That catalog should support:

1. listing all known error templates
2. looking up a template by error code
3. generating documentation pages or sections from the same source of truth
4. validating that error codes are unique within a namespace

This enables a workflow where:

- developers define errors once
- tests assert stable codes
- documentation can enumerate all supported errors
- CLIs and editors can link users to a specific error reference

A likely shape is:

```rust
pub trait ErrorCatalog {
		fn all() -> &'static [ErrorTemplate];
		fn get(code: ErrorCode) -> Option<&'static ErrorTemplate>;
}
```

## Rendering and Serialization Requirements

Hints and template metadata should survive all output modes.

That means:

1. terminal output should show hints in a readable and visually distinct way
2. plain-text output should keep hints without ANSI formatting
3. JSON output should preserve error code, title, message, hints, notes, and suggestions as structured fields

An example JSON shape:

```json
{
	"severity": "error",
	"code": "RBT-E001",
	"title": "Invalid regex pattern",
	"message": "The regex pattern could not be compiled.",
	"hints": [
		"Check that brackets and parentheses are balanced.",
		"Add '^' explicitly if you intend the pattern to match from the current cursor."
	],
	"notes": [],
	"suggestions": []
}
```

## Documentation Generation Goal

The system should make it easy to generate documentation from the catalog.

That documentation may eventually include:

- code
- title
- default message
- default hints
- severity
- examples
- recovery guidance
- links to language-specific notes

This is important because it keeps implementation and documentation aligned.

## Likely Building Blocks

- `Severity`
- `ErrorCode`
- `ErrorTemplate`
- `ErrorCatalog`
- `Diagnostic`
- `Label`
- `Hint`
- `Suggestion`
- `SourcePosition`
- `SourceSpan`
- `DiagnosticSink`
- `TerminalRenderer`
- `JsonRenderer`
- `PlainRenderer`

## Open Questions

1. Should warnings and errors share one top-level type or remain distinct but linked?
2. How should error codes be namespaced across crates?
3. Should collection and emission be combined in one sink abstraction or split?
4. What compatibility guarantees should be made for serialized JSON diagnostics?
5. Should the first implementation support snippets and multi-label spans immediately, or in phases?
6. Should hints be represented as a dedicated field, or should they reuse a more general `help` field internally?
7. How much of the template system should be compile-time/static versus runtime-extensible for language authors?
8. Should documentation generation be part of `rb_common`, or handled by a companion tool that consumes the error catalog?