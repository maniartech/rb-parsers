# Spec: Parser Developer Workflow

**Status**: Conceptual reference — Phase 1 skeleton now; Phase 2 full API
**Purpose**: End-to-end walkthrough of how a developer builds a parser with `rb_parsers`
**Audience**: Grammar authors: anyone building a language, DSL, config format, or query syntax
**Requirement sources**: `docs/requirements/declarative_parser_api_draft.md`,
`docs/requirements/parser-core-semantics.md`,
`docs/requirements/parser-execution-and-consumption-models.md`,
`docs/requirements/parsing-profiles-and-language-modes.md`

---

## Overview

Building a parser with `rb_parsers` follows five layers. Each layer has a clear
responsibility. Layers only depend downward.

```
┌───────────────────────────────────────────────────────────┐
│  5. Consume output                                        │
│     CST traversal  │  AST lowering  │  events  │  incr.  │
├───────────────────────────────────────────────────────────┤
│  4. Compile and run the pipeline                          │
│     DiagnosticsContext   TokenStream   Parser             │
├───────────────────────────────────────────────────────────┤
│  3. Define grammar rules                                  │
│     Grammar<R>  node  field  tok  seq!  one_of!  pratt    │
├───────────────────────────────────────────────────────────┤
│  2. Declare syntax kinds and rule ID enum                 │
│     SyntaxKind constants  (parser-side vocabulary)        │
├───────────────────────────────────────────────────────────┤
│  1. Declare token vocabulary and build the tokenizer      │
│     Token type constants  Tokenizer  scanners             │
└───────────────────────────────────────────────────────────┘
```

Every layer uses only the two below it. The tokenizer knows nothing about the
grammar; the grammar knows nothing about how the output is consumed.

**Phase 1** delivers layers 1 and 4 (pipeline plumbing). Layers 2, 3, and 5 are
the Phase 2 deliverables. This document describes all five layers so grammar
authors have a complete picture of what they are building toward.

---

## Layer 1 — Token vocabulary and tokenizer

The tokenizer is the only place that knows what the source text looks like at
the character level. Everything above it works with token streams.

### 1a. Declare token type constants

Token types are `&'static str` constants. Any string is valid — including
Unicode operator names, keywords from any language, or script-specific symbols.

```rust
// In your crate: my_lang/src/tokens.rs

pub mod my_tok {
    // Literals
    pub const NUMBER:  &str = "Number";
    pub const STRING:  &str = "String";
    pub const TRUE:    &str = "True";
    pub const FALSE:   &str = "False";
    pub const NULL:    &str = "Null";

    // Structural
    pub const LBRACE:  &str = "{";
    pub const RBRACE:  &str = "}";
    pub const LBRACKET: &str = "[";
    pub const RBRACKET: &str = "]";
    pub const COLON:   &str = ":";
    pub const COMMA:   &str = ",";

    // Trivia (optional — register to preserve whitespace in the tree)
    pub const WHITESPACE: &str = "Whitespace";
    pub const COMMENT:    &str = "Comment";
}
```

Token types are your vocabulary. The names you choose here appear in grammar
rules and diagnostics. Use names that are meaningful in your language's context.

### 1b. Build and configure the tokenizer

```rust
use rb_tokenizer::{Tokenizer, TokenizerConfig};
use rb_common::spans::SourceId;

pub fn build_my_tokenizer(source_id: SourceId) -> Tokenizer {
    let mut t = Tokenizer::new()
        .with_source_id(source_id);

    // Register scanners in priority order — first match wins.

    // Keywords — matched with word-boundary check; each gets its own token type
    t.add_keyword_scanner_with_subtypes("Keyword", &[
        ("true",  my_tok::TRUE),
        ("false", my_tok::FALSE),
        ("null",  my_tok::NULL),
    ]);

    // Numbers — hex, binary, octal, float, scientific, underscore separators all handled
    t.add_number_literal_scanner(my_tok::NUMBER, None);

    // Strings — block scanner handles escape sequences inside the delimiters
    t.add_block_scanner(r#"""#, r#"""#, my_tok::STRING, Some(BlockScannerConfig {
        escape_char: Some('\\'),
        ..Default::default()
    }));

    // Structural tokens — exact symbol matching, no regex overhead
    t.add_symbol_scanner("{",  my_tok::LBRACE,   None);
    t.add_symbol_scanner("}",  my_tok::RBRACE,   None);
    t.add_symbol_scanner("[",  my_tok::LBRACKET, None);
    t.add_symbol_scanner("]",  my_tok::RBRACKET, None);
    t.add_symbol_scanner(":",  my_tok::COLON,    None);
    t.add_symbol_scanner(",",  my_tok::COMMA,    None);

    // Whitespace — prefer WhitespaceScanner over a raw regex; choose the mode
    // that matches the language's treatment of newlines:
    //   uniform("Ws")               — all whitespace = one token (JSON, C, Java)
    //   split("Ws", "Nl")           — separate Newline token (Go, JavaScript, Ruby)
    //   with_continuation(...)      — split + backslash-newline = LineContinuation
    t.add_whitespace_scanner(WhitespaceScanner::uniform(my_tok::WHITESPACE));

    t
}
```

**Language-specific keyword boundaries** — different languages define different
characters as part of an identifier.  Use [`WordBoundaryDef`] presets so that
keywords stop matching at the right positions:

```rust
use rb_tokenizer::scanners::{KeywordScanner, WordBoundaryDef};

// Ruby — `save!` and `empty?` must not match the keywords `save` / `empty`
t.add_scanner(Box::new(
    KeywordScanner::new("Keyword", &["def", "end", "do"])
        .with_word_boundary_def(WordBoundaryDef::ruby()),
));

// CSS — `flex-wrap` must not match keyword `flex`
t.add_scanner(Box::new(
    KeywordScanner::new("Keyword", &["flex", "grid"])
        .with_word_boundary_def(WordBoundaryDef::css()),
));
```

**Operator-heavy languages** — use `add_operator_scanner` or `add_operator_scanner_with_subtypes`
for symbolic multi-character operators. Unlike `KeywordScanner`, it does not
enforce a word boundary, so `++`, `+=`, `->`, `=>`, `<<=`, etc. all work
correctly when adjacent to identifiers or digits:

```rust
// All operators share one token_type
t.add_operator_scanner("Op", &["**", "+=", "-=", "++", "--", "+", "-", "*"]);

// Each operator gets its own token_sub_type
t.add_operator_scanner_with_subtypes("Op", &[
    ("<<=", "ShlAssign"),  // longest variants first is fine — scanner sorts internally
    ("<<",  "Shl"),
    ("<=",  "Le"),
    ("<",   "Lt"),
]);
```

**Unicode languages** — use `add_char_class_scanner` for identifier-like tokens;
the lead and continuation specs accept standard ASCII ranges and Unicode character
properties expressed as regex `\p{...}` patterns.

```rust
// Standard ASCII identifier: [a-zA-Z_][a-zA-Z0-9_]*
t.add_char_class_scanner("a-zA-Z_", Some("a-zA-Z0-9_"), "Identifier", None);

// Python-style Unicode identifier (ID_Start / ID_Continue)
t.add_char_class_scanner("\\p{ID_Start}", Some("\\p{ID_Continue}"), "Identifier", None);
```

For languages with Arabic, CJK, Devanagari, or other Unicode script identifiers
use `\p{Script=Arabic}` etc. in the spec strings. The `regex` crate handles all
Unicode scalar values uniformly — no extra setup needed.

**Block scanners** — use for multi-line content where a pair of delimiters
encloses everything:

```rust
// Triple-quoted raw string (like Python or Rust raw strings)
t.add_block_scanner(r#"""""#, r#"""""#, "RawString", None);

// Nested block comments: /* ... /* ... */ ... */
t.add_block_scanner("/*", "*/", "BlockComment", Some(BlockScannerConfig {
    allow_nesting: true,
    ..Default::default()
}));
```

**Indentation-sensitive languages** — use the built-in `IndentationScanner`,
which tracks indent depth and emits `INDENT` / `DEDENT` tokens automatically:

```rust
use rb_tokenizer::scanners::IndentationScanner;

// Register as a contextual scanner; run the pipeline with tokenize_contextual()
t.add_contextual_scanner(Box::new(IndentationScanner::new(
    my_tok::INDENT,   // token type emitted when depth increases
    my_tok::DEDENT,   // token type emitted when depth decreases
)));

// tokenize in contextual mode (threads a ScanContext through all scanners)
let tokens = t.tokenize_contextual(source);
```

**Custom scanners** — implement the `Scanner` trait only for things that none
of the built-in types cover (encoding-dependent syntax, domain-specific binary
chunks, etc.):

```rust
pub struct MySpecialScanner;

impl Scanner for MySpecialScanner {
    fn scan(&mut self, input: &str) -> Option<ScanMatch> {
        // return None to try the next scanner, or ScanMatch { .. } on success
        todo!()
    }
}

t.add_scanner(Box::new(MySpecialScanner));
```

---

## Layer 2 — Syntax kinds and rule IDs

Syntax kinds are the node type vocabulary for the parse tree. They live above
the tokenizer and describe grammatical structure, not raw characters.

### 2a. Declare SyntaxKind constants

```rust
// In your crate: my_lang/src/syntax.rs

use rb_parser::SyntaxKind;

pub mod my_syn {
    use rb_parser::SyntaxKind;

    // Top-level nodes
    pub const VALUE:   SyntaxKind = SyntaxKind::new("Value");
    pub const OBJECT:  SyntaxKind = SyntaxKind::new("Object");
    pub const ARRAY:   SyntaxKind = SyntaxKind::new("Array");
    pub const MEMBER:  SyntaxKind = SyntaxKind::new("Member");

    // Leaf nodes (terminal wrappers)
    pub const STRING:  SyntaxKind = SyntaxKind::new("StringLit");
    pub const NUMBER:  SyntaxKind = SyntaxKind::new("NumberLit");
    pub const TRUE:    SyntaxKind = SyntaxKind::new("TrueLit");
    pub const FALSE:   SyntaxKind = SyntaxKind::new("FalseLit");
    pub const NULL:    SyntaxKind = SyntaxKind::new("NullLit");
}
```

`SyntaxKind::new` is the only thing needed. The name you give it matches what
you see in the CST when debugging or writing visitors.

### 2b. Declare the rule ID enum

Each grammar rule gets an entry in an enum. The enum is the type parameter that
makes `Grammar<R>` type-safe — only valid rule IDs can be passed to `.rule()`
and `ref_()`.

```rust
// In your crate: my_lang/src/grammar.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MyRule {
    Value,
    Object,
    Member,
    Array,
}
```

---

## Layer 3 — Grammar rules (declarative API)

Grammar rules are composed from a small set of combinators. The same rule
definitions drive every output surface (CST, AST lowering, events, incremental)
without rewriting.

### 3a. The combinator vocabulary

| Combinator | What it does |
|---|---|
| `node(kind, rule)` | Wraps `rule` in a named CST node of type `kind`. |
| `field(name, rule)` | Labels the output of `rule` as a named child slot. Accessible by name in the CST and AST lowering. |
| `tok(token_type)` | Matches and consumes exactly one token of the given type. Committed if present. |
| `ref_(rule_id)` | Calls another named rule. Enables recursion. |
| `seq![r1, r2, r3]` | Matches r1 then r2 then r3 in order. Fails softly if r1 misses; commits after r1 succeeds. |
| `one_of![r1, r2, r3]` | Ordered choice. Tries r1 first; if soft-failure then tries r2; etc. Committed failure stops. |
| `between(open, body, close)` | Matches `open`, then `body`, then `close`. Built-in commitment after `open`. Built-in recovery to `close`. |
| `list(element, sep)` | Zero-or-more `element` separated by `sep`. Built-in recovery at separator and closing landmarks. |
| `repeat0(rule)` | Zero or more repetitions of `rule`. Never fails. |
| `repeat1(rule)` | One or more. Soft-fails if zero matches. |
| `pratt(atom).prefix(tok, bp, node).infix_left(tok, bp, node).finish()` | Pratt expression parser. Handles precedence and associativity declaratively. |
| `opt(rule)` | Optional: succeed with `None` if `rule` fails softly. |
| `cut()` | Explicit commitment point. After `cut()` a soft failure becomes a committed failure. |
| `.enabled_if(pred)` | Profile guard. The rule / token is active only when `pred` evaluates true. |
| `.recover_to(any_of![...])` | Override recovery landmark for this rule specifically. |

### 3b. Minimal JSON example

```rust
use rb_parser::{grammar, node, field, tok, ref_, seq, one_of, between, list, Grammar};
use crate::{my_tok, my_syn, MyRule};

pub fn define_my_grammar() -> Grammar<MyRule> {
    grammar()
        // Member: "key": value
        .rule(
            MyRule::Member,
            node(
                my_syn::MEMBER,
                seq![
                    field("key",   tok(my_tok::STRING)),
                    tok(my_tok::COLON),
                    field("value", ref_(MyRule::Value)),
                ],
            ),
        )

        // Object: { member, member, ... }
        .rule(
            MyRule::Object,
            node(
                my_syn::OBJECT,
                between(
                    tok(my_tok::LBRACE),
                    field("members", list(ref_(MyRule::Member), tok(my_tok::COMMA))),
                    tok(my_tok::RBRACE),
                ),
            ),
        )

        // Array: [ value, value, ... ]
        .rule(
            MyRule::Array,
            node(
                my_syn::ARRAY,
                between(
                    tok(my_tok::LBRACKET),
                    field("items", list(ref_(MyRule::Value), tok(my_tok::COMMA))),
                    tok(my_tok::RBRACKET),
                ),
            ),
        )

        // Value: union of all value kinds
        .rule(
            MyRule::Value,
            node(
                my_syn::VALUE,
                one_of![
                    ref_(MyRule::Object),
                    ref_(MyRule::Array),
                    node(my_syn::STRING, tok(my_tok::STRING)),
                    node(my_syn::NUMBER, tok(my_tok::NUMBER)),
                    node(my_syn::TRUE,   tok(my_tok::TRUE)),
                    node(my_syn::FALSE,  tok(my_tok::FALSE)),
                    node(my_syn::NULL,   tok(my_tok::NULL)),
                ],
            ),
        )

        .start(MyRule::Value)
}
```

### 3c. Expression grammar with precedence (Pratt)

For languages with operator precedence and associativity — arithmetic, query
languages, scripting languages:

```rust
pub mod expr_syn {
    use rb_parser::SyntaxKind;
    pub const PROGRAM:  SyntaxKind = SyntaxKind::new("Program");
    pub const LET_STMT: SyntaxKind = SyntaxKind::new("LetStatement");
    pub const EXPR_STMT: SyntaxKind = SyntaxKind::new("ExprStatement");
    pub const BINARY:   SyntaxKind = SyntaxKind::new("BinaryExpr");
    pub const PREFIX:   SyntaxKind = SyntaxKind::new("PrefixExpr");
    pub const GROUP:    SyntaxKind = SyntaxKind::new("GroupExpr");
    pub const INT:      SyntaxKind = SyntaxKind::new("IntLiteral");
    pub const NAME:     SyntaxKind = SyntaxKind::new("Name");
}

pub fn define_expr_grammar() -> Grammar<ExprRule> {
    grammar()
        .rule(
            ExprRule::Program,
            node(expr_syn::PROGRAM, repeat0(ref_(ExprRule::Statement))),
        )
        .rule(
            ExprRule::LetStatement,
            node(
                expr_syn::LET_STMT,
                seq![
                    tok(expr_tok::LET),
                    field("name",  tok(expr_tok::IDENT)),
                    tok(expr_tok::ASSIGN),
                    field("value", ref_(ExprRule::Expr)),
                    tok(expr_tok::SEMI),
                ],
            ),
        )
        .rule(
            ExprRule::Atom,
            one_of![
                node(expr_syn::INT,  tok(expr_tok::INT)),
                node(expr_syn::NAME, tok(expr_tok::IDENT)),
                node(
                    expr_syn::GROUP,
                    between(
                        tok(expr_tok::LPAREN),
                        ref_(ExprRule::Expr),
                        tok(expr_tok::RPAREN),
                    ),
                ),
            ],
        )
        // Pratt table: prefix operators and infix operators with binding power
        .rule(
            ExprRule::Expr,
            pratt(ref_(ExprRule::Atom))
                .prefix(expr_tok::MINUS, 70, node(expr_syn::PREFIX))
                .prefix(expr_tok::NOT,   70, node(expr_syn::PREFIX))
                .infix_left(expr_tok::STAR,  60, node(expr_syn::BINARY))
                .infix_left(expr_tok::SLASH, 60, node(expr_syn::BINARY))
                .infix_left(expr_tok::PLUS,  50, node(expr_syn::BINARY))
                .infix_left(expr_tok::MINUS, 50, node(expr_syn::BINARY))
                .infix_left(expr_tok::EQ,    30, node(expr_syn::BINARY))
                .infix_right(expr_tok::ASSIGN, 10, node(expr_syn::BINARY))
                .finish(),
        )
        .start(ExprRule::Program)
}
```

Direct and indirect left recursion is **rejected at grammar compilation time**
with a diagnostic pointing to the cycle. Use `pratt(...)` for expression
grammars — it is both the safe and the natural choice.

### 3d. Profile-aware rules

When a language has versions, strictness modes, or optional features, guards
keep the grammar compact instead of forked:

```rust
// Trailing comma: only valid in JSON5 / "permissive" profile
list(ref_(MyRule::Member), tok(my_tok::COMMA))
    .trailing_sep(
        tok(my_tok::COMMA)
            .enabled_if(profile().feature("trailing_commas"))
    )

// Line comments: only in the "comments" overlay
node(
    my_syn::COMMENT,
    tok(my_tok::LINE_COMMENT),
)
.enabled_if(profile().feature("comments"))

// v2 string escape: only since version 2
tok(my_tok::UNICODE_ESCAPE)
    .enabled_if(profile().since("v2"))
```

Guards are evaluated once at profile resolution time, not on every token.
The compiled grammar sees a flat, guard-free rule set for that profile variant.

### 3e. Recovery boundaries

Built-in combinators (`between`, `list`, `pratt`) carry sensible default
recovery landmarks. Override only when your grammar genuinely needs custom sync:

```rust
// Recover to the next comma or closing brace inside an object
field(
    "members",
    list(ref_(MyRule::Member), tok(my_tok::COMMA))
        .recover_to(any_of![my_tok::COMMA, my_tok::RBRACE]),
)

// Statement-oriented grammar: recover to next semicolon or closing brace
ref_(ExprRule::Statement)
    .recover_to(any_of![expr_tok::SEMI, expr_tok::RBRACE])
```

For languages where newlines terminate statements (Python, Ruby, Go), register
the newline token and use it as a recovery landmark:

```rust
ref_(MyRule::Statement)
    .recover_to(any_of![my_tok::NEWLINE, my_tok::DEDENT])
```

---

## Layer 4 — Compile and run the pipeline

```rust
use rb_common::diagnostics::DiagnosticsContext;
use rb_common::spans::SourceId;
use rb_tokenizer::pipeline::TokenStream;

pub fn parse_my_lang(source: &str) -> (ParseResult, DiagnosticsContext) {
    // One DiagnosticsContext flows through both layers.
    let mut ctx = DiagnosticsContext::collecting();

    // Step 1: tokenize
    let source_id = SourceId(1);
    let tokenizer = build_my_tokenizer(source_id);
    let tokens = tokenizer.tokenize(source, &mut ctx);

    // Step 2: build token stream (canonical handoff type)
    let stream = TokenStream::new(source, source_id, tokens);

    // Step 3: compile grammar then parse
    //   (grammar.compile() is Phase 2 — returns a compiled, immutable parser)
    let grammar = define_my_grammar();
    let profile = default_profile();
    let parser = grammar.compile(&profile);
    let result = parser.parse_tree(&stream, &mut ctx);   // parse_tree is the default surface

    (result, ctx)
}
```

**One `DiagnosticsContext` for the whole pipeline.** Tokenizer errors (bad
characters, unterminated strings) appear in the same ordered stream as parser
errors (unexpected tokens, missing delimiters). The renderer sees them all
together, in source order.

### Profile resolution

For multi-version or multi-mode languages, resolve the profile before building
tokenizer and parser:

```rust
let profile = profile_catalog
    .request("my-lang")
    .version("v2")
    .mode(ProfileMode::Strict)
    .feature("comments")
    .resolve()?;

let tokenizer = build_my_tokenizer_for(&profile, source_id);
let grammar   = define_my_grammar();
let parser    = grammar.compile(&profile);          // guards evaluated here
```

After `compile(&profile)`, the parser is immutable and can be shared across
threads and reused across parse calls.

---

## Layer 5 — Consume the output

One compiled grammar drives all output surfaces. You choose the surface at call
time, not at grammar-authoring time.

### 5a. CST (default)

The default and easiest surface. The CST preserves all structural boundaries,
delimiters, and recovery artifacts.

```rust
let tree: CstTree = parser.parse_tree(&stream, &mut ctx)?;

// Navigate by kind
for node in tree.children_of_kind(my_syn::OBJECT) {
    for field in node.field("members") {
        println!("{}", field.text_of_child("key"));
    }
}
```

### 5b. AST lowering

AST is a separate transformation over the CST. You control the lowering:

```rust
pub struct JsonValue { /* ... your domain types ... */ }

impl JsonValue {
    pub fn lower(node: &CstNode) -> Result<JsonValue, LoweringError> {
        match node.kind() {
            my_syn::OBJECT => /* lower members */ ,
            my_syn::ARRAY  => /* lower items */,
            my_syn::STRING => Ok(JsonValue::String(node.text().to_owned())),
            my_syn::NUMBER => Ok(JsonValue::Number(node.text().parse()?)),
            my_syn::TRUE   => Ok(JsonValue::Bool(true)),
            my_syn::FALSE  => Ok(JsonValue::Bool(false)),
            my_syn::NULL   => Ok(JsonValue::Null),
            _              => Err(LoweringError::UnexpectedKind(node.kind())),
        }
    }
}

let ast = JsonValue::lower(tree.root())?;
```

AST is always a separate optional step. The CST is always present after parsing.

### 5c. Visitor traversal

```rust
pub struct MyVisitor;

impl TreeVisitor for MyVisitor {
    fn visit_node(&mut self, node: &CstNode, ctx: &mut WalkContext) {
        if node.kind() == my_syn::MEMBER {
            let key = node.field("key").unwrap().text();
            println!("member key: {key}");
        }
        ctx.descend();  // continue into children
    }
}

let mut visitor = MyVisitor;
tree.walk(&mut visitor, WalkOrder::PreOrder);
```

### 5d. Event-based (SAX-like, no tree allocated)

For high-throughput pipelines where you do not need the full tree:

```rust
let events = parser.parse_events(&stream, &mut ctx)?;

for event in events {
    match event {
        ParseEvent::NodeStart { kind, span } => { /* push */ },
        ParseEvent::Token { token_type, span, value } => { /* leaf */ },
        ParseEvent::NodeEnd   { kind }       => { /* pop */ },
        ParseEvent::Error     { diagnostic } => { /* record */ },
    }
}
```

### 5e. Incremental re-parse (editor / REPL)

For editors, language servers, and interactive REPLs where the source changes
incrementally between calls:

```rust
// First parse
let mut session = parser.incremental();
let tree_v1 = session.initial_parse(&stream_v1, &mut ctx)?;

// User edits a small region
let edits = vec![TextEdit::replace(50..60, "new_value")];
let tree_v2 = session.reparse(&stream_v2, &edits, &mut ctx)?;
// Subtrees not touched by the edits are reused as-is
```

### 5f. Render diagnostics

After parsing, render the collected diagnostics in any format:

```rust
use rb_common::render::{render_to_string, RenderOutputPreset};

// Colored terminal output (auto-detects TTY)
let rendered = render_to_string(&ctx, RenderOutputPreset::Auto);
eprintln!("{rendered}");

// CI-friendly plain text (no ANSI codes)
let ci_output = render_to_string(&ctx, RenderOutputPreset::Ci);

// Machine-readable JSON for IDE or tooling integration
let json_output = render_to_string(&ctx, RenderOutputPreset::Machine);
```

---

## Parser behavior guarantees

| Guarantee | Details |
|---|---|
| Deterministic | Same grammar + same input → same tree on every run, regardless of output surface. |
| No unbounded backtracking | Grammars default to linear / near-linear execution. Dangerous prefixes are rejected at compile time. |
| Left recursion rejected | Direct and indirect left recursion produces a compile-time diagnostic with a pointer to the cycle. Use `pratt(...)` for expressions. |
| Commitment is explicit | `between`, `list`, and `pratt` insert safe internal cut points. Use `cut()` explicitly for custom commitment in `seq!`. |
| Recovery is bounded | `RecoveryConfig::max_errors` and `max_recovery_skips` limit how aggressively the parser continues after failures. |
| Thread-safe compiled grammar | The compiled `Parser` is `Send + Sync`. Create once, share across threads. |
| One diagnostic stream | Tokenizer and parser use the same `DiagnosticsContext`. No separate error collections to merge. |

---

## Minimal end-to-end example

A complete JSON parser from scratch, including error output:

```rust
use rb_common::diagnostics::DiagnosticsContext;
use rb_common::render::{render_to_string, RenderOutputPreset};
use rb_common::spans::SourceId;
use rb_tokenizer::pipeline::TokenStream;

fn main() {
    let source = r#"{ "name": "Alice", "age": 30 }"#;

    // 1. Tokenize
    let mut ctx = DiagnosticsContext::collecting();
    let tokenizer = build_my_tokenizer(SourceId(1));
    let tokens = tokenizer.tokenize(source, &mut ctx);

    // 2. Stream
    let stream = TokenStream::new(source, SourceId(1), tokens);

    // 3. Parse (Phase 2 API — shown here for clarity)
    let grammar = define_my_grammar();
    let parser  = grammar.compile(&default_profile());
    let tree    = parser.parse_tree(&stream, &mut ctx).unwrap();

    // 4. Use the tree
    println!("{:#?}", tree);

    // 5. Print any diagnostics
    if ctx.has_errors() {
        eprintln!("{}", render_to_string(&ctx, RenderOutputPreset::Auto));
        std::process::exit(1);
    }
}
```

---

## What each crate owns

| Crate | Owns |
|---|---|
| `rb_common` | `SourceSpan`, `DiagnosticsContext`, `ErrorCatalog`, `Suggestion`, `RecoveryConfig`, all renderers |
| `rb_tokenizer` | `Tokenizer`, `Scanner` trait, `Token`, `TokenStream`, scanner registration, tokenization errors |
| `rb_parser` | `Grammar<R>`, `SyntaxKind`, all combinators, compiled `Parser`, `CstTree`, `TreeVisitor`, `ParseEvent` |
| your crate | Token type constants, `SyntaxKind` constants, rule enum, grammar definition function, AST types, lowering logic |

Grammar authors write **only** what is in the "your crate" row. The framework
provides everything else.

---

## Phase availability summary

| Feature | Phase 1 | Phase 2 |
|---|---|---|
| `Tokenizer`, scanners, token vocab | ✅ | — |
| `Token` with `SourceSpan` | ✅ | — |
| `DiagnosticsContext`, error catalog, renderers | ✅ | — |
| `TokenStream<'src>` handoff type | ✅ | — |
| `Parser` trait skeleton (`parse()` stub) | ✅ | — |
| `SyntaxKind`, combinator vocabulary | — | ✅ |
| `Grammar<R>`, `grammar.compile()` | — | ✅ |
| `CstTree`, `parse_tree()` | — | ✅ |
| `parse_events()`, pull surface | — | ✅ |
| `TreeVisitor`, `Walker` | — | ✅ |
| `pratt(...)` expression parser | — | ✅ |
| Profile guards (`.enabled_if`) | — | ✅ |
| `IncrementalParser` / `session.reparse()` | — | ✅ (or Phase 3) |
| Portable grammar IR, C ABI, Wasm | — | Phase 3 |
