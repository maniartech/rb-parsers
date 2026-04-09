# P2 — Missing Fundamental API

**Priority**: P2 — These are completeness gaps against the declarative parser spec
and framework-objectives. The parser is functional but materially limited in
expressiveness and configurability without these primitives.

**Back to**: [Audit Index](README.md)

---

## C10 · No `look(expr)` (lookahead) or `not(expr)` (negative lookahead)

**Layer**: `rb_parser`
**File**: `crates/rb_parser/src/grammar/combinator.rs`
**Requirement source**: `declarative_parser_api_draft.md` §PEG fundamentals

### Missing API

```rust
// Needed combinators — not yet implemented
pub fn look<R>(expr: impl GrammarRule<R>) -> impl GrammarRule<R>  { /* positive lookahead */ }
pub fn not<R>(expr:  impl GrammarRule<R>) -> impl GrammarRule<R>  { /* negative lookahead */ }
```

### Impact

Without lookahead:

- **Cannot** distinguish keywords from identifiers when both match the same token type
  — a common need in languages where `type`, `class`, `as`, etc. are valid identifiers
  in some positions.
- **Cannot** express `!keyword ~ ident` ("an identifier that is not a keyword").
- **Cannot** implement context-sensitive disambiguation (e.g. `>` as both comparison
  and close-generic at the same position).
- Every grammar that requires any kind of "if the next token looks like X, take this
  path" must resort to the escape hatch of custom `ParseFn` closures, defeating the
  purpose of the declarative combinator API.

### Specification

`look(expr)` attempts `expr` at the current position. If it succeeds, position is
restored (nothing consumed) and the lookahead node succeeds. If `expr` fails, the
lookahead fails without advancing.

`not(expr)` is the inverse. If `expr` succeeds, `not(expr)` fails (without consuming).
If `expr` fails, `not(expr)` succeeds (without consuming).

```rust
// Example: identifier that is not a keyword
let keyword = one_of!(
    tok("Keyword", "if"), tok("Keyword", "while"), tok("Keyword", "return")
);
let non_keyword_ident = seq!(not(keyword), tok("Ident", ""));
```

Both combinators must be integrated into `RuleExpr<R>` as new variants and handled
in `eval()`.

---

## C11 · No `take_until(end_expr)` / `until(end_expr)` combinator

**Layer**: `rb_parser`
**File**: `crates/rb_parser/src/grammar/combinator.rs`
**Requirement source**: `declarative_parser_api_draft.md` §scanning combinators

### Missing API

```rust
pub fn take_until<R>(end: impl GrammarRule<R>) -> impl GrammarRule<R>
pub fn until<R>(content: impl GrammarRule<R>, end: impl GrammarRule<R>) -> impl GrammarRule<R>
```

### Impact

Without `take_until`:

- **Cannot** efficiently scan to a known delimiter for:
  - Raw string literals (scan to `"""` or `'''`)
  - Heredoc bodies (scan to user-specified end marker)
  - Embedded expression end (`}` in `"hello ${name} world"`)
  - Line comments (scan to `\n`)
  - Block doc comments with custom end markers
- The workaround is `many(not(end) ~ any_token)` which requires one full `eval` round-trip per token,
  producing enormous recursive call stacks for large string literals.

### Specification

`take_until(end)` matches and consumes tokens until `end` would match at the current
position. Does not consume `end`. Always succeeds (can match zero tokens).

```rust
// Example: JS template literal body
let template_body = take_until(tok("TemplatePunct", "}"));
let interpolation  = seq!(tok("TemplatePunct", "${"), expr, tok("TemplatePunct", "}"));
let template_lit   = seq!(tok("Quote", "`"), many(alt(interpolation, template_body)), tok("Quote", "`"));
```

`until(content, end)` is the higher-level version: matches zero or more of `content`
until `end` is seen, then consumes `end`.

---

## C13 · `PrattOp` only discriminates by `token_type`, not `token_sub_type`

**Layer**: `rb_parser`
**File**: `crates/rb_parser/src/grammar/combinator.rs`
**Requirement source**: `declarative_parser_api_draft.md` §Pratt parsing

### Current API

```rust
pub struct PrattOp {
    pub token_type: &'static str,
    pub binding_power: (Option<u8>, Option<u8>),
}
```

### Problem

A typical tokenizer assigns all binary operators the same `token_type = "Op"` but
differentiates them via `token_sub_type`:

- `"=="` → `token_type: "Op"`, `token_sub_type: Some("Eq")`
- `"!="` → `token_type: "Op"`, `token_sub_type: Some("NotEq")`
- `"<="` → `token_type: "Op"`, `token_sub_type: Some("Lte")`

If `PrattOp` only matches on `token_type`, all `"Op"` tokens receive the same binding
power, making `a + b == c + d` parse as `a + (b == (c + d))` — a typical arithmetic
grammar will be completely wrong.

Alternatively, if each operator must be given its own `token_type` to work around this
limitation, the tokenizer becomes cluttered with dozens of individual operator-type
names.

### Fix

```rust
pub struct PrattOp {
    pub token_type:     &'static str,
    pub token_sub_type: Option<&'static str>,   // None = match any sub_type
    pub binding_power:  (Option<u8>, Option<u8>),
}
```

The Pratt matching check becomes:

```rust
fn matches_op(op: &PrattOp, token: &Token) -> bool {
    token.token_type == op.token_type
    && op.token_sub_type.map_or(true, |st| Some(st) == token.token_sub_type.as_deref())
}
```

---

## C14 · No `CompiledParser::with_recovery(config)` builder

**Layer**: `rb_parser`
**File**: `crates/rb_parser/src/grammar/combinator.rs`
**Requirement source**: `framework-objectives.md` §Error Recovery

### Missing API

```rust
// Needed — not implemented
impl CompiledParser {
    pub fn with_recovery(self, config: RecoveryConfig) -> Self { ... }
}
```

### Impact

The `RecoveryConfig` on `CompiledParser` exists and is passed to the parse engine, but
there is no public API to change it after compilation. The only way to get a parser
with custom recovery settings is to set them before `Grammar::compile()`, which is
not how the API is intended to work (grammar compilation is expensive and should
happen once; recovery configuration is per-parse-session).

In practice this means:

- Every parser uses the default `max_recovery_steps` (whatever the hardcoded default
  is) regardless of context.
- Test authors cannot set `max_recovery_steps = 0` to disable recovery in unit tests.
- Production code cannot set a tighter limit to prevent runaway recovery on malformed
  input.

### Fix

```rust
impl CompiledParser {
    pub fn with_recovery(mut self, config: RecoveryConfig) -> Self {
        self.recovery = config;
        self
    }
    pub fn with_max_recovery_steps(mut self, steps: usize) -> Self {
        self.recovery.max_recovery_steps = steps;
        self
    }
}
```

---

## C15 · No grammar introspection API

**Layer**: `rb_parser`
**File**: `crates/rb_parser/src/grammar/combinator.rs`
**Requirement source**: `framework-objectives.md` §IDE Tooling

### Missing API

```rust
// Needed — not implemented
impl CompiledParser {
    /// Returns the set of token types that can legally start parsing at
    /// the current position given the parse stack state.
    pub fn completion_tokens(&self, partial_cst: &PartialCst, position: SourcePosition) -> Vec<TokenType>;

    /// Returns the FIRST set of a named grammar rule.
    pub fn first_set(&self, rule: R) -> HashSet<TokenType>;

    /// Returns the FOLLOW set of a named grammar rule.
    pub fn follow_set(&self, rule: R) -> HashSet<TokenType>;
}
```

### Impact

Without grammar introspection:

- **LSP completion** cannot suggest valid completions — it has no way to ask the
  parser "what tokens are valid at byte offset X?".
- **Error messages** cannot suggest "expected one of: `{`, `[`, `)`, `if`, `while`" —
  only generic "unexpected token" is possible.
- **Grammar testing** cannot verify that a rule has the expected first set, making it
  harder to catch ambiguity or missing alternatives.

The FIRST and FOLLOW sets of a PEG grammar are computable at `Grammar::compile()` time.
Storing them in `CompiledGrammar` adds negligible overhead to compilation and unlocks
all the above use cases.

---

## C16 · `one_of!` macro expands to O(N) left-nested `alt2` tree

**Layer**: `rb_parser`
**File**: `crates/rb_parser/src/grammar/combinator.rs`
**Requirement source**: `declarative_parser_api_draft.md` §performance notes

### Root Cause

```rust
macro_rules! one_of {
    ($a:expr, $b:expr) => { alt2($a, $b) };
    ($a:expr, $($rest:tt)+) => { alt2($a, one_of!($($rest)+)) };
}
```

`one_of!(a, b, c, d, e)` expands to:
```
alt2(a, alt2(b, alt2(c, alt2(d, e))))
```

At parse time, this is evaluated as a sequential left-to-right linear search. If the
correct branch is `e` and there are 20 alternatives, the parser makes 19 failing
`eval` calls before reaching `e`. This makes `one_of!` O(N choices) in the worst case.

### Impact

Large `one_of!` blocks (common for statement lists, expression starts, and binary
operator trees) experience O(N) backtracking per position. A `one_of!(20 statement
types)` at the start of every statement makes statement parsing 5–20× slower than
necessary.

### Fix

At `Grammar::compile()` time, compute the FIRST token-type set for each alternative
in every `Alt2` node. When evaluating `Alt2`, dispatch by first token type rather than
trying each alternative:

```rust
// CompiledGrammarNode — after compile()
enum CompiledExpr {
    // ...
    Alt {
        alternatives: Vec<CompiledExpr>,
        /// Map from first-token-type to alternative index for O(1) dispatch
        first_token_dispatch: HashMap<TokenType, usize>,
        /// Alternatives whose first set could not be determined statically
        fallback: Vec<usize>,
    }
}
```

The dispatch becomes `O(1)` for unambiguous alternatives and falls back to sequential
search only for ambiguous or catch-all branches.
