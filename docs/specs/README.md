# rb_parsers — Implementation Specs

This directory contains **implementation-ready specifications** for the
`rb_parsers` workspace.

These are distinct from the design discussion documents in `docs/requirements/`.
Each spec here has:

- all significant open questions resolved with explicit decisions
- exact Rust type definitions, trait signatures, and module layouts
- implementation notes and integration patterns
- a clear dependency order for implementation sequencing

---

## Phase 1 — Diagnostics Infrastructure and Pipeline Contract

**Scope**: `rb_common` (new), `rb_tokenizer` (upgrade), `rb_parser` (skeleton)
**Status**: Fully specified. No open design questions. Implement in order.

### Language Universality Guarantee

Phase 1 is intentionally language-agnostic. After implementing these 12 specs a
developer can build a tokenizer and parser skeleton for **any human language or
programming language**, including:

| Language family | How Phase 1 supports it |
|---|---|
| ASCII / Latin-script languages | Regex scanners with standard patterns |
| Unicode identifiers (Arabic, CJK, Devanagari, etc.) | Rust `regex` crate supports `\p{ID_Start}`, `\p{ID_Continue}`, `\p{Script=…}` out of the box |
| Indentation-sensitive (Python, YAML, Haskell) | Use the built-in `IndentationScanner::new(indent_type, dedent_type)` registered via `add_contextual_scanner`; run the pipeline with `tokenize_contextual()` |
| Multi-line / raw strings (Rust, Lua, C++) | `BlockScanner` handles arbitrary delimiter pairs including raw heredoc markers |
| Operator-heavy languages (APL, Haskell, C++) | `OperatorScanner` matches multi-character operators longest-first with no word-boundary check; `token_type` is `&'static str` — any Unicode symbol is a valid token type string |
| Languages with significant whitespace | `WhitespaceScanner::split("Whitespace", "Newline")` emits a distinct `Newline` token the parser can use for ASI; `IndentationScanner` handles full INDENT/DEDENT for indentation-sensitive languages |
| Mixed-script source (emoji, math symbols, etc.) | `SourceSpan` byte offsets are aligned to Unicode scalar value boundaries; display helpers convert to char counts |
| Multiple source files / interactive REPL | `SourceId` distinguishes each buffer; `TokenStream` carries the buffer's `source_id` |

**No language-specific token types, keywords, or grammar rules are baked into
any Phase 1 component.** Everything is registered by the developer at runtime.

### Phase 1 Spec Sequence

#### rb_common — new modules

| # | Spec | Module | Depends on |
|---|---|---|---|
| 1 | [Source Spans and Labels](source-spans-and-labels.md) | `rb_common::spans` | nothing |
| 2 | [Environment Detection](environment-detection.md) | `rb_common::env` | nothing |
| 3 | [Error Catalog and Compatibility](error-catalog-and-compatibility.md) | `rb_common::catalog` | nothing |
| 4 | [Suggestions and Fixes](suggestions-and-fixes.md) | `rb_common::suggestions` | `spans` |
| 5 | [Recovery and Error Boundaries](recovery-and-error-boundaries.md) | `rb_common::recovery` | `spans` |
| 6 | [Diagnostics Runtime](diagnostics-runtime.md) | `rb_common::diagnostics` | `spans`, `catalog`, `suggestions` |
| 7 | [Renderers and Output](renderers-and-output.md) | `rb_common::render` | `diagnostics`, `env`, `spans` |
| 8 | [Automatic Hinting](automatic-hinting.md) | `rb_common::hinting` | `diagnostics`, `spans`, `recovery` |

#### rb_tokenizer — upgrades

| # | Spec | Crate | Depends on |
|---|---|---|---|
| 9 | [Token Type Upgrade](rb_tokenizer-token-upgrade.md) | `rb_tokenizer::tokens` | spec 1 (`spans`) |
| 10 | [Error Catalog (RBT)](rb_tokenizer-error-catalog.md) | `rb_tokenizer::catalog` | spec 3 (`catalog`) |
| 11 | [Diagnostics Integration](rb_tokenizer-diagnostics-integration.md) | `rb_tokenizer::tokenizers` | specs 9, 10, 6 |

#### Pipeline contract

| # | Spec | Crates | Depends on |
|---|---|---|---|
| 12 | [Tokenizer–Parser Pipeline Contract](tokenizer-parser-pipeline-contract.md) | `rb_tokenizer`, `rb_parser` | specs 9, 11, 6 |

#### Developer reference

| — | [Parser Developer Workflow](parser-developer-workflow.md) | all crates | Read this for the end-to-end grammar authoring guide |

---

### Phase 1 Deliverables

When all 12 specs above are implemented the workspace will have:

**`rb_common`**
- `rb_common::spans` — `SourceId`, `SourcePosition`, `SourceSpan`, `DiagnosticLocation`, `SpanLabel`, `ContextScope`, `DiagnosticContextRegion`, `SnippetRequest`
- `rb_common::env` — `EnvironmentSnapshot`, `EnvironmentDetector` trait, `RealEnvironmentDetector`, `FixedEnvironmentDetector`
- `rb_common::catalog` — `ErrorCode`, `ErrorSeverity`, `ErrorTemplate`, `ErrorCatalog` trait, `StaticErrorCatalog`, `CompositeErrorCatalog`
- `rb_common::suggestions` — `EditKind`, `Applicability`, `TextEdit`, `Suggestion`
- `rb_common::recovery` — `RecoveryMode`, `RecoveryConfig`, `RecoveryBoundaryKind`, `RecoveryBoundary`, `RecoveryBoundarySet`, `RecoveryAction`, `RecoveryState`, `RecoveredMarker<T>`, `ErrorBudget`
- `rb_common::diagnostics` — `Hint`, `Diagnostic`, `DiagnosticBuilder`, `DiagnosticsMode`, `DiagnosticSink` trait, `NullSink`, `CollectingSink`, `HookSink`, `CompositeSink`, `DiagnosticsContext`, `SeverityPolicy`
- `rb_common::render` — `OutputFormat`, `RenderTarget`, `RenderOptions`, `RenderOutputPreset`, `RenderRequest`, `RendererSuitability`, `DiagnosticRenderer` trait, `RendererSelector`, `TerminalRenderer`, `PlainRenderer`, `JsonRenderer`, `render_to_string()`
- `rb_common::hinting` — `HintConfidence`, `HintOrigin`, `HintCandidate`, `HintContext`, `HintProvider` trait, `HintPipeline`, `HintFilter`, `ExpectedVsFoundProvider`, `DelimiterMismatchProvider`, `default_hint_pipeline()`

**`rb_tokenizer`** (upgraded)
- `Token` gains `span: SourceSpan`; `line`/`column` fields removed; `display_line()` / `display_column()` convenience methods added
- `Tokenizer` gains `source_id: SourceId` field and `with_source_id()` builder
- `tokenize()` signature becomes `tokenize(&self, input: &str, ctx: &mut DiagnosticsContext) -> Vec<Token>`
- `tokenize_contextual()` threads a `ScanContext` through all contextual scanners for mode-aware lexing
- `last_errors()` removed; callers use `ctx.collected()` instead
- `RegexScanner` no longer uses `eprintln!`; normalization warnings go through `DiagnosticsContext`
- `rb_tokenizer::catalog` module with `RBT_CATALOG` and `codes::*` constants (human-readable slugs: `RBT-unrecognized-char`, `RBT-unmatched-block`, `RBT-invalid-pattern`, `RBT-unterminated-block`, `RBT-pattern-auto-anchored`, `RBT-error-limit-reached`)
- `rb_tokenizer::pipeline::TokenStream<'src>` — the canonical tokenizer output / parser input
- **New scanner types added beyond the original `SymbolScanner`, `RegexScanner`, `BlockScanner`, `EolScanner`, `ClosureScanner`:**
  - `KeywordScanner` — reserved words with word-boundary enforcement; boundary definition pluggable via `WordBoundaryDef`; `add_keyword_scanner` / `add_keyword_scanner_with_subtypes`
  - `CharClassScanner` — identifier-style tokens using lead + continuation char-class specs (ASCII ranges or Unicode `\p{...}`); `add_char_class_scanner`
  - `NumberLiteralScanner` — complete numeric literal lexing (decimal, hex, binary, octal, float, scientific notation, underscore separators); `add_number_literal_scanner`
  - `OperatorScanner` — longest-match multi-character operator scanning without word-boundary check (covers `++`, `+=`, `->`, `<<=`, etc.); `add_operator_scanner` / `add_operator_scanner_with_subtypes`
  - `WhitespaceScanner` — configurable whitespace handling: `uniform` (all whitespace as one token), `split` (separate `Newline` token for ASI languages), `with_continuation` (also emits backslash-newline as `LineContinuation`); `add_whitespace_scanner`
  - `WordBoundaryDef` — reusable, clone-able word-boundary definition for `KeywordScanner`; named presets: `ruby()` (`?!`), `javascript()` (`$`), `css()` (`-`), `r_lang()` (`.` `$`), `lisp()` (`?!+-*/<>=`), `haskell()` (`'`)
  - `IndentationScanner` — significant-whitespace INDENT/DEDENT emission; registered via `add_contextual_scanner`, runs under `tokenize_contextual()`
  - `ContextualScanner` trait — mode-switching scanner interface receiving `&mut ScanContext`; `add_contextual_scanner` / `add_contextual_closure`
  - `ScanContext` — shared mutable state threaded through all contextual scanners in one `tokenize_contextual()` call
  - `BinaryScanner` trait + `BinaryTokenizer` — byte-level scanner API for binary file formats

**`rb_parser`** (skeleton)
- `ParseResult` struct
- `Parser` trait with `parse(&self, stream: &TokenStream<'_>, ctx: &mut DiagnosticsContext) -> ParseResult`

**Observable user-facing behaviour**
- Tokenization errors carry byte-exact `SourceSpan`s with 0-based storage and 1-based display helpers; byte offsets are Unicode-safe (always aligned to a scalar value boundary)
- All tokenizer errors have stable, human-readable slug codes (`RBT-unrecognized-char`, `RBT-unmatched-block`, …) — readable without consulting any documentation
- Tokenizer and parser share one `DiagnosticsContext`; lexical errors appear before syntax errors in emission order
- Diagnostics can be collected silently, emitted immediately, or both
- The same `Diagnostic` value renders as colored terminal output, plain text, or JSON
- Automatic hints from `DelimiterMismatchProvider` and `ExpectedVsFoundProvider` are available to tokenizer and parser without extra wiring
- No `eprintln!` or `println!` in any library code

---

## Phase 2 — Parser Core

**Scope**: `rb_parser` (full implementation), `rb_common` (`RBP` catalog)
**Status**: Fully specified. All design decisions resolved. Implement in order.

> **See [Parser Developer Workflow](parser-developer-workflow.md) for the full
> end-to-end guide** — combinator vocabulary, grammar examples for JSON and
> expression languages, profile guards, recovery boundaries, and all five
> output surfaces. Reading that document first gives context for everything below.

### What developers write (brief summary)

Phase 2 exposes a grammar authoring API. A developer builds a parser by:

1. Declaring `SyntaxKind` constants — the node type vocabulary for the parse tree.
2. Declaring a rule ID enum — type-safe rule references (`impl RuleId`).
3. Writing `Grammar<R>` rules using a small set of composable combinators.
4. Calling `grammar.compile(&profile)` to get a `CompiledParser`.
5. Calling `parser.parse_tree(&stream, &mut ctx)` to get a `CstTree`.

The core combinator set:

| Combinator | Purpose |
|---|---|
| `node(kind, rule)` | named CST boundary |
| `field(name, rule)` | named child slot |
| `tok(token_type)` | consume one token |
| `ref_(rule_id)` | call another rule |
| `seq![r1, r2, ...]` | ordered sequence; commits after first succeeds |
| `one_of![r1, r2, ...]` | PEG ordered choice |
| `between(open, body, close)` | delimited content; commits after `open`; built-in recovery |
| `list(element, sep)` | separated list; commits at each `sep`; built-in recovery |
| `repeat0(r)` / `repeat1(r)` | zero-or-more / one-or-more repetition |
| `pratt(atom).prefix(...).infix_left(...).finish()` | precedence-climbing expressions |
| `.enabled_if(profile()...)` | profile / version / feature guard |
| `.recover_to(any_of![...])` | custom recovery landmark |
| `cut()` | explicit commitment point |

### Phase 2 Spec Sequence

| # | Spec | Module | Depends on |
|---|---|---|---|
| 13 | [Parser Error Catalog (RBP)](rb_parser-error-catalog.md) | `rb_parser::catalog` | Phase 1 spec 3 (`rb_common::catalog`) |
| 14 | [Parsing Profiles and Language Modes](rb_parser-profiles.md) | `rb_parser::profiles` | nothing |
| 15 | [CST Layout and Syntax Tree](rb_parser-cst-layout.md) | `rb_parser::cst` | Phase 1 spec 1 (`rb_common::spans`) |
| 16 | [Parser Engine Semantics](rb_parser-engine-semantics.md) | `rb_parser::engine` | specs 13, 14, 15 |
| 17 | [Grammar API and Combinator Vocabulary](rb_parser-grammar-api.md) | `rb_parser::grammar` | specs 15, 16 |
| 18 | [Parser Consumption Surfaces](rb_parser-consumption-surfaces.md) | `rb_parser`, `rb_parser::events`, `rb_parser::visitors` | specs 15, 16, 17 |

### Phase 2 Deliverables

When all 6 specs above are implemented the workspace will have a complete
`rb_parser` that:

- Accepts a `TokenStream` from `rb_tokenizer` and a `DiagnosticsContext`
- Compiles `Grammar<R>` declarations into an immutable, `Send+Sync` `CompiledParser`
- Detects left recursion, unreachable branches, and conflicting guards **at compile time**
- Runs PEG-style ordered choice with commitment (no unbounded backtracking)
- Emits `RBP-*` diagnostics through the shared `DiagnosticsContext`
- Returns a compact arena-based `CstTree` as the default output
- Also supports event streaming (`Vec<ParseEvent>`) and custom `ParseStrategy` outputs
- Provides a depth-first `TreeVisitor` traversal API
- Supports an `IncrementalParser` for editor-style reparsing with edit hints
- Resolves language profiles through `ProfileCatalog` with directional compatibility rules

---

## Phase 3 — Portability Layer (Future: lower priority)

Phase 3 covers the portable grammar IR and multi-target backends.

Blocked by: Phase 2 grammar combinator stabilization.

Candidates:
- `rb_common::portable_ir` — `PortableGrammarIr` struct and portable subset rules
- C ABI packaging surface for Rust-backed cross-language use
- WebAssembly / WASI packaging
- Portable grammar IR serialization format

See `docs/requirements/portable-grammar-ir-and-multi-target-backends.md`.

---

## Non-Goals for All Phases

Never in scope regardless of phase:

- evaluator / semantic execution (belongs in user crates above `rb_parser`)
- host-language-specific semantic actions baked into the portable grammar layer
- opaque AI-based automatic hint generation in the core framework

---

## Design Principles Carried Forward

All specs in this directory honor these invariants:

- **No hidden global state** — environment detection returns owned snapshots; diagnostics contexts are passed explicitly as `&mut`
- **Test determinism** — every public type supports testing without real terminal or file-system access; use `FixedEnvironmentDetector` and `CollectingSink` in tests
- **Zero-cost statics** — error templates and error codes are `'static` and zero-heap-allocation in the common path
- **Emission order** — diagnostic streams preserve strict emission order; severity grouping is a renderer concern only
- **Silence over filler** — automatic hints below `HintConfidence::Medium` are suppressed, never padded with generic advice
- **No direct printing** — no `eprintln!` or `println!` in any library crate; all output goes through `DiagnosticsContext`
