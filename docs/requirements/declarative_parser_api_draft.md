# Declarative Parser API Draft

This document shows the full user-facing declaration style if token identity is owned by `rb-tokenizer` and reused directly by `rb-parser`.

The code below is a draft of the intended public API shape. It is not wired to the current implementation yet. The point is to show the authoring model end-to-end with no omitted enums, constants, or helper functions.

## Profile-aware direction

This draft should eventually support resolved parsing profiles so one language can expose combinations such as:

- `v1`
- `v1 + strict`
- `v2`
- `v2 + comments`

The intended direction is:

```rust
let profile = profile_catalog
    .request("json")
    .version("v1")
    .mode(ProfileMode::Strict)
    .resolve()?;

let tokenizer = build_json_tokenizer(&profile);
let parser = build_json_parser(&profile);
```

Rules and scanners that differ by version, strictness, or feature flags should preferably be guarded declaratively by the resolved profile rather than forcing grammar authors to fork whole parser definitions.

The shared design for this lives in `rb_common/docs/specs/parsing-profiles-and-language-modes.md`.

## Multiple parser surfaces direction

This draft should eventually support multiple consumption styles without forcing grammar authors to define separate parsers for each one.

The intended direction is:

- tree-oriented parsing should be the default public experience
- visitor-style traversal should work over produced syntax structures
- event and pull surfaces should be possible for advanced consumers
- incremental parsing should remain a design goal for editor and tooling scenarios

The overall API goal is:

- super easy default usage
- a fast path for common parsers
- more serious parser behavior available with a little more care
- the same rule-based model extended upward rather than replaced

Strategy-based design is likely a good fit for advanced structures and traversal, but common usage should not require explicit strategy selection.

Conceptually:

```rust
let parser = build_json_parser(&profile);

let tree = parser.parse_tree(&tokens)?;
let events = parser.parse_events(&tokens)?;
let stream = parser.parse_pull(&tokens);
```

Common usage should stay smaller than that. Advanced surfaces should be additive.

The longer-term direction may look like:

```rust
let tree = parser.parse_tree(&tokens)?;
let ast = parser.parse_with_strategy(&tokens, AstStrategy::default())?;
let stream = parser.parse_with_strategy(&tokens, EventStrategy::default())?;
```

But these strategies should be layered behind small defaults rather than becoming mandatory setup for every parser.

The likely type-shape direction is:

- keep one core `Parser` or compiled grammar surface for ordinary parsing
- allow AST output through lowering or materialization strategies rather than a separate `AstParser`
- allow event and pull outputs through dedicated methods or strategies rather than separate parser families
- allow incremental parsing through a distinct stateful runtime such as `IncrementalParser` or `IncrementalSession` if reuse state needs to live across edits

Conceptually:

```rust
let parser = build_json_parser(&profile);

let tree = parser.parse_tree(&tokens)?;
let ast = parser.parse_with_strategy(&tokens, AstStrategy::default())?;

let mut incremental = parser.incremental();
let updated = incremental.reparse(&tokens, &edits)?;
```

This keeps the simple path simple while still making the long-lived incremental case explicit.

## CST-first default direction

The current design direction is that the first stable parse result should be a CST.

That means:

- `parse_tree` should be the default and most obvious parser surface
- AST should be produced by lowering or materialization on top of the CST
- visitor and traversal APIs should work naturally over CST, with AST visitors added later where useful
- incremental reuse should be designed around CST stability rather than AST stability

The key constraint is performance: this CST-first default must be compact and efficient enough that the easy path remains fast.

That means the framework should avoid:

- duplicated source text in tree nodes
- eager CST plus eager AST materialization by default
- object-heavy tree layouts that make ordinary parsing uncompetitive

The shared design for this lives in `rb_common/docs/specs/syntax-tree-and-materialization.md`.

The exact API may differ, but one grammar definition should be able to power these surfaces without forcing duplicated grammar declarations.

The shared design for this lives in `rb_common/docs/specs/parser-execution-and-consumption-models.md`.

## Novice ergonomics direction

The default authoring path should be small enough that a novice Rust programmer can build a real parser without first learning parser-engine internals.

That means:

- the starter grammar vocabulary should stay small: `node`, `field`, `tok`, `ref_`, `seq!`, `one_of!`, `between`, `list`, and `pratt`
- built-in combinators should carry strong defaults for commitment, recovery, and diagnostics in common cases
- compile-time diagnostics should point out unsupported recursion, unreachable branches, and obviously risky grammar shapes early
- default compilation should choose performance-safe execution behavior rather than the most general behavior
- advanced strategies should extend the same grammar model instead of introducing a second authoring model

The shared design for parser execution semantics should live in `rb_common/docs/specs/parser-core-semantics.md`.

## Backend-neutral direction

The long-term direction should minimize repeated language-porting effort.

That means one language definition should eventually be able to drive:

- the Rust runtime backend used by this workspace
- future emitted or generated parsers in other host languages
- backend-specific packaging only where a target runtime genuinely needs it

The key architectural rule is that the canonical grammar model should stay backend-neutral.

That means:

- grammar structure, syntax kinds, precedence, profile guards, and recovery boundaries should lower into a portable grammar IR
- Rust should remain the first and richest reference backend while the IR stabilizes
- future non-Rust targets should compile from the same normalized grammar description rather than from duplicated grammar authoring
- host-language-specific semantic actions should not become the default way to express syntax meaning in the canonical grammar layer

Where practical, portability should prefer Rust-backed delivery surfaces first:

- direct Rust library output
- C bindings for native embedding
- WebAssembly for browser or embedder scenarios
- WASI for sandboxed or component-oriented deployment

Host-native emitted parser source should remain available when required, but it should not be the only portability story.

Conceptually:

```rust
let ir = define_json_grammar().lower_to_ir(&profile)?;

let rust_parser = RustBackend::default().compile(&ir)?;
let c_api = CAbiBackend::default().package(&ir)?;
let wasm_module = WasmBackend::default().package(&ir)?;
```

The exact API may differ, but the direction matters: grammar authoring should stay portable first, with host-specific escape hatches kept explicit and clearly non-portable.

Evaluation and interpreter frameworks should remain a later layer on top of this. They are usually less portable than parsing and should not distort the core grammar model prematurely.

The shared design for this should live in `rb_common/docs/specs/portable-grammar-ir-and-multi-target-backends.md`.

## Suggested structure-first syntax

If the same declarative grammar should drive CST, AST lowering, event output, and incremental parsing, then the grammar layer should describe syntax structure first, not domain AST values directly.

That suggests a syntax built around a small set of structural primitives:

- `node(kind, rule)` to create a syntax node boundary
- `field(name, rule)` to label an important child slot
- `tok(token_kind)` to consume a token structurally
- `ref_(rule_id)` to reference another grammar rule
- existing combinators such as `seq!`, `one_of!`, `between`, `list`, and `pratt`

In this model, the grammar describes the parse shape once, and different consumers decide what to do with it later.

That same separation is also what makes future multi-target emission plausible: structure-first grammars are much easier to normalize into a portable IR than grammars that depend on host-language closures in the rule body.

Conceptually:

```rust
pub mod json_syn {
    use rb_parser::SyntaxKind;

    pub const VALUE: SyntaxKind = SyntaxKind::new("JsonValue");
    pub const OBJECT: SyntaxKind = SyntaxKind::new("JsonObject");
    pub const MEMBER: SyntaxKind = SyntaxKind::new("JsonMember");
    pub const ARRAY: SyntaxKind = SyntaxKind::new("JsonArray");
    pub const STRING: SyntaxKind = SyntaxKind::new("JsonString");
    pub const NUMBER: SyntaxKind = SyntaxKind::new("JsonNumber");
    pub const TRUE: SyntaxKind = SyntaxKind::new("JsonTrue");
    pub const FALSE: SyntaxKind = SyntaxKind::new("JsonFalse");
    pub const NULL: SyntaxKind = SyntaxKind::new("JsonNull");
}

pub fn define_json_grammar() -> Grammar<JsonRule> {
    grammar()
        .rule(
            JsonRule::Member,
            node(
                json_syn::MEMBER,
                seq![
                    field("key", tok(json_tok::STRING)),
                    tok(json_tok::COLON),
                    field("value", ref_(JsonRule::Value)),
                ],
            ),
        )
        .rule(
            JsonRule::Object,
            node(
                json_syn::OBJECT,
                between(
                    tok(json_tok::LBRACE),
                    field("members", list(ref_(JsonRule::Member), tok(json_tok::COMMA))),
                    tok(json_tok::RBRACE),
                ),
            ),
        )
        .rule(
            JsonRule::Array,
            node(
                json_syn::ARRAY,
                between(
                    tok(json_tok::LBRACKET),
                    field("items", list(ref_(JsonRule::Value), tok(json_tok::COMMA))),
                    tok(json_tok::RBRACKET),
                ),
            ),
        )
        .rule(
            JsonRule::Value,
            node(
                json_syn::VALUE,
                one_of![
                    ref_(JsonRule::Object),
                    ref_(JsonRule::Array),
                    node(json_syn::STRING, tok(json_tok::STRING)),
                    node(json_syn::NUMBER, tok(json_tok::NUMBER)),
                    node(json_syn::TRUE, tok(json_tok::TRUE)),
                    node(json_syn::FALSE, tok(json_tok::FALSE)),
                    node(json_syn::NULL, tok(json_tok::NULL)),
                ],
            ),
        )
        .start(JsonRule::Value)
}
```

The same grammar can then power multiple surfaces:

```rust
let grammar = define_json_grammar();
let parser = grammar.compile(&profile);

let tree = parser.parse_tree(&tokens)?;
let ast = JsonAst::lower(&tree)?;
let events = parser.parse_events(&tokens)?;

let mut incremental = parser.incremental();
let updated_tree = incremental.reparse(&tokens, &edits)?;
```

In that model:

- grammar defines syntax structure
- CST is the default parse product
- AST is a separate lowering step
- events are an alternate consumer of the same parse behavior
- incremental parsing reuses the same grammar and parse model with a stateful runtime

## Suggested profile-guard syntax

Profile-aware rule differences should stay local, readable, and easy to scan.

A likely direction is chainable guard syntax on rule expressions:

```rust
tok(json_tok::COMMA)
    .enabled_if(profile().since("v2").or_feature("trailing_commas"));

node(json_syn::COMMENT, tok(json_tok::COMMENT))
    .enabled_if(profile().feature("comments"));
```

This keeps profile logic close to the rule it affects without forcing grammar authors to wrap large grammar regions in imperative branching.

## Suggested recovery-boundary syntax

Recovery configuration should also stay close to the grammar region that owns the boundary.

A likely direction is chainable recovery syntax on rule expressions:

```rust
field(
    "members",
    list(ref_(JsonRule::Member), tok(json_tok::COMMA))
        .recover_to(any_of![json_tok::COMMA, json_tok::RBRACE]),
)
```

For statement-oriented grammars, the same style should work with different landmarks:

```rust
ref_(ExprRule::Statement)
    .recover_to(any_of![expr_tok::SEMI, expr_tok::RBRACE]);
```

Common combinators such as `between`, `list`, and `pratt` should ship with sensible default boundaries so explicit `recover_to(...)` remains an advanced refinement rather than mandatory ceremony.

## Expression syntax in the same model

The same idea should also work for precedence-driven grammars.

Conceptually:

```rust
pub mod expr_syn {
    use rb_parser::SyntaxKind;

    pub const PROGRAM: SyntaxKind = SyntaxKind::new("Program");
    pub const LET_STMT: SyntaxKind = SyntaxKind::new("LetStatement");
    pub const EXPR_STMT: SyntaxKind = SyntaxKind::new("ExprStatement");
    pub const NAME: SyntaxKind = SyntaxKind::new("Name");
    pub const INT: SyntaxKind = SyntaxKind::new("IntLiteral");
    pub const GROUP: SyntaxKind = SyntaxKind::new("GroupExpr");
    pub const PREFIX: SyntaxKind = SyntaxKind::new("PrefixExpr");
    pub const BINARY: SyntaxKind = SyntaxKind::new("BinaryExpr");
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
                    field("name", tok(expr_tok::IDENT)),
                    tok(expr_tok::ASSIGN),
                    field("value", ref_(ExprRule::Expr)),
                    tok(expr_tok::SEMI),
                ],
            ),
        )
        .rule(
            ExprRule::ExprStatement,
            node(
                expr_syn::EXPR_STMT,
                seq![field("expr", ref_(ExprRule::Expr)), tok(expr_tok::SEMI)],
            ),
        )
        .rule(
            ExprRule::Atom,
            one_of![
                node(expr_syn::INT, tok(expr_tok::INT)),
                node(expr_syn::NAME, tok(expr_tok::IDENT)),
                node(
                    expr_syn::GROUP,
                    between(tok(expr_tok::LPAREN), ref_(ExprRule::Expr), tok(expr_tok::RPAREN)),
                ),
            ],
        )
        .rule(
            ExprRule::Expr,
            pratt(ref_(ExprRule::Atom))
                .prefix(expr_tok::MINUS, 70, node(expr_syn::PREFIX))
                .infix_left(expr_tok::STAR, 60, node(expr_syn::BINARY))
                .infix_left(expr_tok::SLASH, 60, node(expr_syn::BINARY))
                .infix_left(expr_tok::PLUS, 50, node(expr_syn::BINARY))
                .infix_left(expr_tok::MINUS, 50, node(expr_syn::BINARY))
                .finish(),
        )
        .start(ExprRule::Program)
}
```

## What changes from the current AST-shaped examples

The main shift is this:

- current examples use `.map(...)` and `.try_map(...)` in the core grammar to construct domain values directly
- the structure-first version uses `node(...)`, `field(...)`, and `tok(...)` in the core grammar instead
- AST construction moves into a separate lowering phase that reads the CST

That is the change that makes one declarative grammar reusable across CST, AST, events, pull parsing, and incremental parsing.

## Shared token identity in `rb-tokenizer`

```rust
use rb_tokenizer::{TokenKind, Tokenizer};
use rb_tokenizer::tokens::Token;
use rb_parser::prelude::*;
```

`TokenKind` is owned by `rb-tokenizer`, not by `rb-parser`.

```rust
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct TokenKind {
    pub kind: &'static str,
    pub sub_kind: Option<&'static str>,
}

impl TokenKind {
    pub const fn new(kind: &'static str) -> Self {
        Self {
            kind,
            sub_kind: None,
        }
    }

    pub const fn sub(kind: &'static str, sub_kind: &'static str) -> Self {
        Self {
            kind,
            sub_kind: Some(sub_kind),
        }
    }
}
```

## Complete JSON example

### JSON token constants

```rust
pub mod json_tok {
    use rb_tokenizer::TokenKind;

    pub const STRING: TokenKind = TokenKind::new("String");
    pub const NUMBER: TokenKind = TokenKind::new("Number");

    pub const LBRACE: TokenKind = TokenKind::sub("Brace", "OpenBrace");
    pub const RBRACE: TokenKind = TokenKind::sub("Brace", "CloseBrace");
    pub const LBRACKET: TokenKind = TokenKind::sub("Bracket", "OpenBracket");
    pub const RBRACKET: TokenKind = TokenKind::sub("Bracket", "CloseBracket");

    pub const COLON: TokenKind = TokenKind::new("Colon");
    pub const COMMA: TokenKind = TokenKind::new("Comma");

    pub const TRUE: TokenKind = TokenKind::sub("Literal", "True");
    pub const FALSE: TokenKind = TokenKind::sub("Literal", "False");
    pub const NULL: TokenKind = TokenKind::sub("Literal", "Null");
}
```

### JSON tokenizer

```rust
pub fn build_json_tokenizer() -> Tokenizer {
    Tokenizer::builder()
        .symbol(json_tok::LBRACE, "{")
        .symbol(json_tok::RBRACE, "}")
        .symbol(json_tok::LBRACKET, "[")
        .symbol(json_tok::RBRACKET, "]")
        .symbol(json_tok::COLON, ":")
        .symbol(json_tok::COMMA, ",")
        .regex(json_tok::STRING, r#"^"([^"\\]|\\.)*""#)
        .regex(json_tok::NUMBER, r"^-?\d+(\.\d+)?([eE][-+]?\d+)?")
        .regex(json_tok::TRUE, r"^true\b")
        .regex(json_tok::FALSE, r"^false\b")
        .regex(json_tok::NULL, r"^null\b")
        .skip_whitespace()
        .track_positions(true)
        .build()
}
```

### JSON syntax kinds

```rust
pub mod json_syn {
    use rb_parser::SyntaxKind;

    pub const VALUE: SyntaxKind = SyntaxKind::new("JsonValue");
    pub const OBJECT: SyntaxKind = SyntaxKind::new("JsonObject");
    pub const MEMBER: SyntaxKind = SyntaxKind::new("JsonMember");
    pub const ARRAY: SyntaxKind = SyntaxKind::new("JsonArray");
    pub const STRING: SyntaxKind = SyntaxKind::new("JsonString");
    pub const NUMBER: SyntaxKind = SyntaxKind::new("JsonNumber");
    pub const TRUE: SyntaxKind = SyntaxKind::new("JsonTrue");
    pub const FALSE: SyntaxKind = SyntaxKind::new("JsonFalse");
    pub const NULL: SyntaxKind = SyntaxKind::new("JsonNull");
}
```

### JSON AST lowering target

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue {
    Object(Vec<(String, JsonValue)>),
    Array(Vec<JsonValue>),
    String(String),
    Number(f64),
    Boolean(bool),
    Null,
}
```

### JSON parser rule ids

```rust
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum JsonRule {
    Value,
    Object,
    Member,
    Array,
}
```

### JSON grammar

```rust
pub fn define_json_grammar() -> Grammar<JsonRule> {
    grammar()
        .rule(
            JsonRule::Member,
            node(
                json_syn::MEMBER,
                seq![
                    field("key", tok(json_tok::STRING)),
                    tok(json_tok::COLON),
                    field("value", ref_(JsonRule::Value)),
                ],
            ),
        )
        .rule(
            JsonRule::Object,
            node(
                json_syn::OBJECT,
                between(
                    tok(json_tok::LBRACE),
                    field("members", list(ref_(JsonRule::Member), tok(json_tok::COMMA))),
                    tok(json_tok::RBRACE),
                ),
            )
        )
        .rule(
            JsonRule::Array,
            node(
                json_syn::ARRAY,
                between(
                    tok(json_tok::LBRACKET),
                    field("items", list(ref_(JsonRule::Value), tok(json_tok::COMMA))),
                    tok(json_tok::RBRACKET),
                ),
            )
        )
        .rule(
            JsonRule::Value,
            node(
                json_syn::VALUE,
                one_of![
                    ref_(JsonRule::Object),
                    ref_(JsonRule::Array),
                    node(json_syn::STRING, tok(json_tok::STRING)),
                    node(json_syn::NUMBER, tok(json_tok::NUMBER)),
                    node(json_syn::TRUE, tok(json_tok::TRUE)),
                    node(json_syn::FALSE, tok(json_tok::FALSE)),
                    node(json_syn::NULL, tok(json_tok::NULL)),
                ],
            ),
        )
        .start(JsonRule::Value)
}

pub fn build_json_parser(profile: &ResolvedParseProfile) -> Parser {
    define_json_grammar().compile(profile)
}
```

### JSON AST lowering

```rust
fn decode_json_string(raw: &str) -> String {
    let inner = raw
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .unwrap_or(raw);

    inner
        .replace(r#"\\""#, "\"")
        .replace(r#"\\n"#, "\n")
        .replace(r#"\\r"#, "\r")
        .replace(r#"\\t"#, "\t")
        .replace(r#"\\\\"#, "\\")
}

pub struct JsonAst;

impl JsonAst {
    pub fn lower(tree: &SyntaxTree) -> Result<JsonValue, LoweringError> {
        // Walk the CST using syntax kinds and named fields, then construct JsonValue.
        todo!()
    }
}
```

### JSON end-to-end usage

```rust
pub fn parse_json(
    source: &str,
    profile: &ResolvedParseProfile,
) -> Result<JsonValue, ParsePipelineError> {
    let tokenizer = build_json_tokenizer();
    let parser = build_json_parser(profile);

    let tokens = tokenizer
        .tokenize(source)
        .map_err(ParsePipelineError::from_tokenizer)?;

    let tree = parser
        .parse_tree(&tokens)
        .map_err(ParsePipelineError::from_parser)?;

    JsonAst::lower(&tree).map_err(ParsePipelineError::from_lowering)
}

fn json_demo() {
    let profile = profile_catalog.resolve_named("json/v1")?;
    let source = r#"{"name":"rb-parser","items":[1,2,3],"ok":true}"#;
    let value = parse_json(source, &profile).unwrap();
    println!("{value:#?}");
}
```

## Complete expression example with precedence

### Expression token constants

```rust
pub mod expr_tok {
    use rb_tokenizer::TokenKind;

    pub const LET: TokenKind = TokenKind::sub("Keyword", "Let");
    pub const IDENT: TokenKind = TokenKind::new("Identifier");
    pub const INT: TokenKind = TokenKind::new("Integer");

    pub const ASSIGN: TokenKind = TokenKind::sub("Operator", "Assign");
    pub const PLUS: TokenKind = TokenKind::sub("Operator", "Plus");
    pub const MINUS: TokenKind = TokenKind::sub("Operator", "Minus");
    pub const STAR: TokenKind = TokenKind::sub("Operator", "Star");
    pub const SLASH: TokenKind = TokenKind::sub("Operator", "Slash");

    pub const LPAREN: TokenKind = TokenKind::sub("Delimiter", "LParen");
    pub const RPAREN: TokenKind = TokenKind::sub("Delimiter", "RParen");
    pub const SEMI: TokenKind = TokenKind::sub("Delimiter", "Semi");
}
```

### Expression tokenizer

```rust
pub fn build_expr_tokenizer() -> Tokenizer {
    Tokenizer::builder()
        .regex(expr_tok::LET, r"^let\b")
        .regex(expr_tok::IDENT, r"^[A-Za-z_][A-Za-z0-9_]*")
        .regex(expr_tok::INT, r"^\d+")
        .symbol(expr_tok::ASSIGN, "=")
        .symbol(expr_tok::PLUS, "+")
        .symbol(expr_tok::MINUS, "-")
        .symbol(expr_tok::STAR, "*")
        .symbol(expr_tok::SLASH, "/")
        .symbol(expr_tok::LPAREN, "(")
        .symbol(expr_tok::RPAREN, ")")
        .symbol(expr_tok::SEMI, ";")
        .skip_whitespace()
        .track_positions(true)
        .build()
}
```

### Expression syntax kinds

```rust
pub mod expr_syn {
    use rb_parser::SyntaxKind;

    pub const PROGRAM: SyntaxKind = SyntaxKind::new("Program");
    pub const STATEMENT: SyntaxKind = SyntaxKind::new("Statement");
    pub const LET_STMT: SyntaxKind = SyntaxKind::new("LetStatement");
    pub const EXPR_STMT: SyntaxKind = SyntaxKind::new("ExprStatement");
    pub const INT: SyntaxKind = SyntaxKind::new("IntLiteral");
    pub const NAME: SyntaxKind = SyntaxKind::new("NameExpr");
    pub const GROUP: SyntaxKind = SyntaxKind::new("GroupExpr");
    pub const PREFIX: SyntaxKind = SyntaxKind::new("PrefixExpr");
    pub const BINARY: SyntaxKind = SyntaxKind::new("BinaryExpr");
}
```

### Expression AST lowering target

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    Negate,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Int(i64),
    Name(String),
    Prefix {
        op: UnaryOp,
        rhs: Box<Expr>,
    },
    Binary {
        lhs: Box<Expr>,
        op: BinaryOp,
        rhs: Box<Expr>,
    },
    Group(Box<Expr>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Let {
        name: String,
        value: Expr,
    },
    Expr(Expr),
}
```

### Expression parser rule ids

```rust
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum ExprRule {
    Program,
    Statement,
    LetStatement,
    ExprStatement,
    Expr,
    Atom,
}
```

### Expression grammar

```rust
pub fn define_expr_grammar() -> Grammar<ExprRule> {
    grammar()
        .rule(
            ExprRule::Program,
            node(expr_syn::PROGRAM, repeat0(ref_(ExprRule::Statement))),
        )
        .rule(
            ExprRule::Statement,
            node(
                expr_syn::STATEMENT,
                one_of![
                    ref_(ExprRule::LetStatement),
                    ref_(ExprRule::ExprStatement),
                ],
            ),
        )
        .rule(
            ExprRule::LetStatement,
            node(
                expr_syn::LET_STMT,
                seq![
                    tok(expr_tok::LET),
                    field("name", tok(expr_tok::IDENT)),
                    tok(expr_tok::ASSIGN),
                    field("value", ref_(ExprRule::Expr)),
                    tok(expr_tok::SEMI),
                ],
            ),
        )
        .rule(
            ExprRule::ExprStatement,
            node(
                expr_syn::EXPR_STMT,
                seq![field("expr", ref_(ExprRule::Expr)), tok(expr_tok::SEMI)],
            ),
        )
        .rule(
            ExprRule::Atom,
            one_of![
                node(expr_syn::INT, tok(expr_tok::INT)),
                node(expr_syn::NAME, tok(expr_tok::IDENT)),
                node(
                    expr_syn::GROUP,
                    between(tok(expr_tok::LPAREN), ref_(ExprRule::Expr), tok(expr_tok::RPAREN)),
                ),
            ],
        )
        .rule(
            ExprRule::Expr,
            pratt(ref_(ExprRule::Atom))
                .prefix(expr_tok::MINUS, 70, node(expr_syn::PREFIX))
                .infix_left(expr_tok::STAR, 60, node(expr_syn::BINARY))
                .infix_left(expr_tok::SLASH, 60, node(expr_syn::BINARY))
                .infix_left(expr_tok::PLUS, 50, node(expr_syn::BINARY))
                .infix_left(expr_tok::MINUS, 50, node(expr_syn::BINARY))
                .finish(),
        )
        .start(ExprRule::Program)
}

pub fn build_expr_parser(profile: &ResolvedParseProfile) -> Parser {
    define_expr_grammar().compile(profile)
}
```

### Expression AST lowering

```rust
pub struct ExprAst;

impl ExprAst {
    pub fn lower(tree: &SyntaxTree) -> Result<Vec<Stmt>, LoweringError> {
        // Walk CST structure and lower it into Vec<Stmt>.
        todo!()
    }
}
```

### Expression end-to-end usage

```rust
pub fn parse_program(
    source: &str,
    profile: &ResolvedParseProfile,
) -> Result<Vec<Stmt>, ParsePipelineError> {
    let tokenizer = build_expr_tokenizer();
    let parser = build_expr_parser(profile);

    let tokens = tokenizer
        .tokenize(source)
        .map_err(ParsePipelineError::from_tokenizer)?;

    let tree = parser
        .parse_tree(&tokens)
        .map_err(ParsePipelineError::from_parser)?;

    ExprAst::lower(&tree).map_err(ParsePipelineError::from_lowering)
}

fn expr_demo() {
    let profile = profile_catalog.resolve_named("expr/v1")?;
    let source = r#"
        let result = 1 + 2 * (3 - 4);
        result + 10;
    "#;

    let program = parse_program(source, &profile).unwrap();
    println!("{program:#?}");
}
```

## What this draft is trying to prove

1. Token constants live in `rb-tokenizer` and are reused directly by the parser.
2. Parser declarations are based on constants and enums, not string names.
3. The same declarative grammar can describe syntax structure once and then power CST, AST lowering, events, and incremental parsing.
4. JSON uses plain PEG-style structure while remaining CST-first.
5. Expressions use a dedicated precedence builder instead of awkward recursive precedence ladders.
6. The user-facing authoring model stays compact even though the runtime can still be optimized internally.
