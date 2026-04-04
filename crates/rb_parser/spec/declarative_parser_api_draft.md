# Declarative Parser API Draft

This document shows the full user-facing declaration style if token identity is owned by `rb-tokenizer` and reused directly by `rb-parser`.

The code below is a draft of the intended public API shape. It is not wired to the current implementation yet. The point is to show the authoring model end-to-end with no omitted enums, constants, or helper functions.

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

### JSON AST

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

### JSON helper

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
```

### JSON grammar

```rust
pub fn build_json_parser() -> Grammar<JsonRule, JsonValue> {
    grammar()
        .rule(
            JsonRule::Member,
            seq![json_tok::STRING, json_tok::COLON, ref_(JsonRule::Value)].map(
                |(key_token, _, value)| {
                    (decode_json_string(&key_token.value), value)
                },
            ),
        )
        .rule(
            JsonRule::Object,
            between(
                json_tok::LBRACE,
                list(ref_(JsonRule::Member), json_tok::COMMA),
                json_tok::RBRACE,
            )
            .map(JsonValue::Object),
        )
        .rule(
            JsonRule::Array,
            between(
                json_tok::LBRACKET,
                list(ref_(JsonRule::Value), json_tok::COMMA),
                json_tok::RBRACKET,
            )
            .map(JsonValue::Array),
        )
        .rule(
            JsonRule::Value,
            one_of![
                ref_(JsonRule::Object),
                ref_(JsonRule::Array),
                json_tok::STRING
                    .map(|token: &Token| JsonValue::String(decode_json_string(&token.value))),
                json_tok::NUMBER
                    .try_map(|token: &Token| token.value.parse::<f64>().map(JsonValue::Number)),
                json_tok::TRUE.value(JsonValue::Boolean(true)),
                json_tok::FALSE.value(JsonValue::Boolean(false)),
                json_tok::NULL.value(JsonValue::Null),
            ],
        )
        .start(JsonRule::Value)
}
```

### JSON end-to-end usage

```rust
pub fn parse_json(source: &str) -> Result<JsonValue, ParseError> {
    let tokenizer = build_json_tokenizer();
    let parser = build_json_parser();

    let tokens = tokenizer
        .tokenize(source)
        .map_err(ParseError::from_tokenizer)?;

    parser.parse(&tokens)
}

fn json_demo() {
    let source = r#"{"name":"rb-parser","items":[1,2,3],"ok":true}"#;
    let value = parse_json(source).unwrap();
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

### Expression AST

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
pub fn build_expr_parser() -> Grammar<ExprRule, Vec<Stmt>> {
    grammar()
        .rule(
            ExprRule::Program,
            repeat0(ref_(ExprRule::Statement)),
        )
        .rule(
            ExprRule::Statement,
            one_of![
                ref_(ExprRule::LetStatement),
                ref_(ExprRule::ExprStatement),
            ],
        )
        .rule(
            ExprRule::LetStatement,
            seq![
                expr_tok::LET,
                expr_tok::IDENT,
                expr_tok::ASSIGN,
                ref_(ExprRule::Expr),
                expr_tok::SEMI,
            ]
            .map(|(_, name_token, _, value, _)| Stmt::Let {
                name: name_token.value.clone(),
                value,
            }),
        )
        .rule(
            ExprRule::ExprStatement,
            seq![ref_(ExprRule::Expr), expr_tok::SEMI]
                .map(|(expr, _)| Stmt::Expr(expr)),
        )
        .rule(
            ExprRule::Atom,
            one_of![
                expr_tok::INT
                    .try_map(|token: &Token| token.value.parse::<i64>().map(Expr::Int)),
                expr_tok::IDENT.map(|token: &Token| Expr::Name(token.value.clone())),
                between(expr_tok::LPAREN, ref_(ExprRule::Expr), expr_tok::RPAREN)
                    .map(|expr| Expr::Group(Box::new(expr))),
            ],
        )
        .rule(
            ExprRule::Expr,
            pratt(ref_(ExprRule::Atom))
                .prefix(expr_tok::MINUS, 70, |_, rhs| Expr::Prefix {
                    op: UnaryOp::Negate,
                    rhs: Box::new(rhs),
                })
                .infix_left(expr_tok::STAR, 60, |lhs, _, rhs| Expr::Binary {
                    lhs: Box::new(lhs),
                    op: BinaryOp::Multiply,
                    rhs: Box::new(rhs),
                })
                .infix_left(expr_tok::SLASH, 60, |lhs, _, rhs| Expr::Binary {
                    lhs: Box::new(lhs),
                    op: BinaryOp::Divide,
                    rhs: Box::new(rhs),
                })
                .infix_left(expr_tok::PLUS, 50, |lhs, _, rhs| Expr::Binary {
                    lhs: Box::new(lhs),
                    op: BinaryOp::Add,
                    rhs: Box::new(rhs),
                })
                .infix_left(expr_tok::MINUS, 50, |lhs, _, rhs| Expr::Binary {
                    lhs: Box::new(lhs),
                    op: BinaryOp::Subtract,
                    rhs: Box::new(rhs),
                })
                .finish(),
        )
        .start(ExprRule::Program)
}
```

### Expression end-to-end usage

```rust
pub fn parse_program(source: &str) -> Result<Vec<Stmt>, ParseError> {
    let tokenizer = build_expr_tokenizer();
    let parser = build_expr_parser();

    let tokens = tokenizer
        .tokenize(source)
        .map_err(ParseError::from_tokenizer)?;

    parser.parse(&tokens)
}

fn expr_demo() {
    let source = r#"
        let result = 1 + 2 * (3 - 4);
        result + 10;
    "#;

    let program = parse_program(source).unwrap();
    println!("{program:#?}");
}
```

## What this draft is trying to prove

1. Token constants live in `rb-tokenizer` and are reused directly by the parser.
2. Parser declarations are based on constants and enums, not string names.
3. JSON uses plain PEG-style structure.
4. Expressions use a dedicated precedence builder instead of awkward recursive precedence ladders.
5. The user-facing authoring model stays compact even though the runtime can still be optimized internally.
