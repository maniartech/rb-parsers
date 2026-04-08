# Spec: Grammar API and Combinator Vocabulary

**Status**: Ready for implementation
**Module**: `rb_parser::grammar`
**Depends on**: `rb_parser::cst`, `rb_parser::engine`, `rb_parser::profiles`
**Requirement source**: `docs/requirements/declarative_parser_api_draft.md`,
`docs/requirements/parser-core-semantics.md`

---

## Decisions Made

| Question | Decision |
|---|---|
| Starter vocabulary | Nine primitives: `node`, `field`, `tok`, `ref_`, `seq!`, `one_of!`, `between`, `list`, `pratt`. Plus `repeat0`, `repeat1`, `opt`, `cut`, `any_of!` (recovery helper). |
| Profile guards | Chainable `.enabled_if(guard)` method on every combinator. Guards do not change what the grammar does — they activate or deactivate a rule branch based on the resolved profile. |
| Recovery boundaries | Chainable `.recover_to(landmarks)` method. Common combinators (`between`, `list`, `pratt`) provide sensible built-in recovery so explicit `.recover_to(...)` is an advanced refinement, not mandatory. |
| Grammar compilation | `Grammar<R>::compile(&profile)` returns `Result<CompiledParser, GrammarError>`. Compile-time checks catch left recursion, unreachable branches, and conflicting guards before any input is parsed. |
| Rule identifier type | Any type implementing the `RuleId` marker trait. Recommended: a `#[derive(Copy, Clone, Debug)]` enum per grammar. |
| `GrammarRule` is a trait | All combinators return `impl GrammarRule<R>`. The concrete types are internal; the public API surface is the trait + the free functions. |
| `pratt` builder | A dedicated `PrattBuilder<R>` fluent API rather than a single function with many parameters. This keeps per-operator metadata close to the operator declaration. |

---

## Module Layout

```
rb_parser::grammar
├── RuleId              (marker trait)
├── GrammarRule         (combinator trait)
├── GrammarBuilder<R>
├── Grammar<R>
├── CompiledParser
├── GrammarError
├── ProfileGuardBuilder
├── RecoveryLandmarks
├── PrattBuilder<R>
├── node()
├── field()
├── tok()
├── ref_()
├── seq![]
├── one_of![]
├── any_of![]           (recovery landmark helper, not a combinator)
├── between()
├── list()
├── repeat0()
├── repeat1()
├── opt()
├── cut()
└── pratt()
```

---

## Core Traits

### RuleId

A marker trait for grammar rule identifier types. Typically implemented by
a user-defined `enum`. The bounds ensure rules can be used as hash map keys
and printed in error messages.

```rust
pub trait RuleId: Copy + Clone + std::fmt::Debug + std::hash::Hash + Eq + 'static {}
```

---

### GrammarRule<R>

The trait implemented by every combinator return type. The `Sized + 'static`
bound is required because rules are stored in a hash map inside `Grammar<R>`.

The two most important methods are `enabled_if` and `recover_to`; combinators
chain them to attach a profile guard or recovery boundary without ceremony.

```rust
pub trait GrammarRule<R: RuleId>: Sized + 'static {
    /// Wraps this rule in a profile guard.
    /// The rule is a no-op (acts like a soft failure) when `guard.is_active(profile)`
    /// returns `false` during parsing.
    fn enabled_if(self, guard: RuleProfileGuard) -> impl GrammarRule<R>;

    /// Wraps this rule in an error-recovery boundary.
    /// When this rule produces a `CommittedFailure`, the engine advances the
    /// cursor to the next token that matches `landmarks`, emits a diagnostic,
    /// and resumes parsing.
    fn recover_to(self, landmarks: RecoveryLandmarks) -> impl GrammarRule<R>;
}
```

---

## Free Combinator Functions

All combinator functions are free functions in `rb_parser::grammar`. They are
re-exported from `rb_parser::prelude` for ergonomic use.

### `node`

Wraps a rule in a syntax node boundary. The CST builder will open and close a
`CstNode` of kind `kind` around the output of `rule`.

```rust
pub fn node<R: RuleId>(
    kind: SyntaxKind,
    rule: impl GrammarRule<R>,
) -> impl GrammarRule<R>
```

---

### `field`

Labels the output of `rule` with a field name. The field name is recorded in
`CstNodeChild::field_name` for named-child access.

```rust
pub fn field<R: RuleId>(
    name: &'static str,
    rule: impl GrammarRule<R>,
) -> impl GrammarRule<R>
```

---

### `tok`

Matches and consumes one token of the given type. Returns a `SoftFailure` when
the next token's kind does not match.

```rust
pub fn tok<R: RuleId>(token_type: &'static str) -> impl GrammarRule<R>
```

For tokens with sub-kinds use `tok_sub`:

```rust
pub fn tok_sub<R: RuleId>(
    token_type: &'static str,
    sub_kind: &'static str,
) -> impl GrammarRule<R>
```

---

### `ref_`

References another rule by its `RuleId`. Enables recursive grammars and
avoids cloning entire rule expressions for shared sub-patterns.

```rust
pub fn ref_<R: RuleId>(rule_id: R) -> impl GrammarRule<R>
```

---

### `seq!` macro

Matches a sequence of rules in order. All rules must succeed for `seq!` to
succeed. Commits after the first rule succeeds (see engine-semantics for the
exact commitment rule).

```rust
/// seq![rule0, rule1, rule2, ...]
/// Expands to nested Seq combinators at macro-expansion time.
/// At runtime: tries rule0; if it succeeds, commits and requires rule1, etc.
#[macro_export]
macro_rules! seq {
    ($rule:expr) => { $rule };
    ($first:expr, $($rest:expr),+) => {
        $crate::grammar::seq2($first, seq!($($rest),+))
    };
}
```

---

### `one_of!` macro

PEG ordered choice. Tries each alternative left-to-right and returns the
first `Success`. A `SoftFailure` in one alternative causes the next to be
tried. A `CommittedFailure` propagates upward immediately without trying
further alternatives.

```rust
/// one_of![rule0, rule1, rule2, ...]
#[macro_export]
macro_rules! one_of {
    ($rule:expr) => { $rule };
    ($first:expr, $($rest:expr),+) => {
        $crate::grammar::alt2($first, one_of!($($rest),+))
    };
}
```

---

### `any_of!` macro (recovery landmarks)

Constructs a `RecoveryLandmarks` set from a list of token type strings. Used
with `.recover_to(...)` rather than as a combinator.

```rust
/// any_of![tok_type0, tok_type1, ...]
/// Returns a RecoveryLandmarks value (not a GrammarRule).
#[macro_export]
macro_rules! any_of {
    ($($tok:expr),+) => {
        $crate::grammar::RecoveryLandmarks::from_token_types(&[$($tok),+])
    };
}
```

---

### `between`

Matches `open`, then `body`, then `close`. Commits after `open` succeeds.
Provides built-in recovery to `close` if `body` or `close` fail.

```rust
pub fn between<R: RuleId>(
    open:  impl GrammarRule<R>,
    body:  impl GrammarRule<R>,
    close: impl GrammarRule<R>,
) -> impl GrammarRule<R>
```

**Automatic recovery**: If `body` or `close` produce a `CommittedFailure`,
the engine advances to the next `close` token and emits `RBP-unmatched-delimiter`.

---

### `list`

Zero-or-more occurrences of `element` separated by `sep`. Does not require a
trailing separator. Commits after each `sep` matches (a subsequent element is
mandatory after a separator).

```rust
pub fn list<R: RuleId>(
    element: impl GrammarRule<R>,
    sep:     impl GrammarRule<R>,
) -> impl GrammarRule<R>
```

To require at least one element, wrap with `seq![element, list(...)]` or use
`list1` (which requires at least one occurrence):

```rust
pub fn list1<R: RuleId>(
    element: impl GrammarRule<R>,
    sep:     impl GrammarRule<R>,
) -> impl GrammarRule<R>
```

---

### `repeat0` / `repeat1`

Matches zero or more (`repeat0`) / one or more (`repeat1`) occurrences of
`rule`. Each successful match is accumulated into the child list of the
enclosing `node`.

```rust
pub fn repeat0<R: RuleId>(rule: impl GrammarRule<R>) -> impl GrammarRule<R>
pub fn repeat1<R: RuleId>(rule: impl GrammarRule<R>) -> impl GrammarRule<R>
```

`repeat0` never fails (zero occurrences is valid). `repeat1` returns a
`SoftFailure` if the first attempt fails.

---

### `opt`

Optionally matches `rule`. Returns `Success` with no children if `rule`
returns `SoftFailure`. Converts `CommittedFailure` to `CommittedFailure`
(commitment is not cleared by `opt`).

```rust
pub fn opt<R: RuleId>(rule: impl GrammarRule<R>) -> impl GrammarRule<R>
```

---

### `cut`

Explicit commitment. All subsequent failures in the enclosing rule become
`CommittedFailure`. Use when the grammar has passed a point of no return that
automatic commitment rules do not cover.

```rust
pub fn cut<R: RuleId>() -> impl GrammarRule<R>
```

Example:

```rust
seq![
    tok(json_tok::STRING),
    cut(), // after seeing a key string, we are committed to "key: value"
    tok(json_tok::COLON),
    ref_(JsonRule::Value),
]
```

---

### `pratt`

Builds a Pratt (top-down operator precedence) expression parser. Returns a
`PrattBuilder<R>` for registering prefix, infix, and postfix operators.

```rust
pub fn pratt<R: RuleId>(atom: impl GrammarRule<R>) -> PrattBuilder<R>
```

#### `PrattBuilder<R>`

```rust
pub struct PrattBuilder<R: RuleId> { /* opaque */ }

impl<R: RuleId> PrattBuilder<R> {
    /// A unary prefix operator with binding power `bp`.
    /// `node_wrapper` is a `node(kind)` call used to wrap the result.
    pub fn prefix(
        self,
        token_type: &'static str,
        bp: u8,
        node_wrapper: impl GrammarRule<R>,
    ) -> Self;

    /// A left-associative binary infix operator with binding power `bp`.
    pub fn infix_left(
        self,
        token_type: &'static str,
        bp: u8,
        node_wrapper: impl GrammarRule<R>,
    ) -> Self;

    /// A right-associative binary infix operator with binding power `bp`.
    pub fn infix_right(
        self,
        token_type: &'static str,
        bp: u8,
        node_wrapper: impl GrammarRule<R>,
    ) -> Self;

    /// A unary postfix operator with binding power `bp`.
    pub fn postfix(
        self,
        token_type: &'static str,
        bp: u8,
        node_wrapper: impl GrammarRule<R>,
    ) -> Self;

    /// Finalizes the Pratt builder and returns a `GrammarRule`.
    pub fn finish(self) -> impl GrammarRule<R>;
}
```

Commits after the first infix operator is consumed (see engine-semantics).

---

## Grammar Builder API

### `grammar` free function

Entry point. Returns a fresh `GrammarBuilder<R>`.

```rust
pub fn grammar<R: RuleId>() -> GrammarBuilder<R>
```

### `GrammarBuilder<R>`

```rust
pub struct GrammarBuilder<R: RuleId> { /* opaque */ }

impl<R: RuleId> GrammarBuilder<R> {
    /// Register a rule. Each `rule_id` must be registered exactly once;
    /// a second call with the same `rule_id` returns an error at compile time.
    pub fn rule(self, rule_id: R, rule: impl GrammarRule<R>) -> Self;

    /// Designate the start rule and finalise the builder into a `Grammar<R>`.
    /// Returns `Err` if `start_rule` has not been registered.
    pub fn start(self, start_rule: R) -> Grammar<R>;
}
```

### `Grammar<R>`

```rust
pub struct Grammar<R: RuleId> { /* opaque */ }

impl<R: RuleId> Grammar<R> {
    /// Compile the grammar against a resolved profile.
    ///
    /// Performs:
    /// 1. Left-recursion cycle check.
    /// 2. Unreachable branch detection.
    /// 3. Conflicting guard detection.
    /// 4. Profile guard evaluation to prune unreachable branches for this profile.
    ///
    /// Returns a `CompiledParser` ready to parse token streams.
    pub fn compile(
        self,
        profile: &ResolvedProfile,
    ) -> Result<CompiledParser, GrammarError>;

    /// Like `compile` but enables memoization for the named rules.
    /// Only rule IDs whose names (via `Debug`) appear in `memo_rules` are
    /// memoized. Panics if any named rule is not registered.
    pub fn compile_with_memo(
        self,
        profile: &ResolvedProfile,
        memo_rules: &[R],
    ) -> Result<CompiledParser, GrammarError>;
}
```

---

## GrammarError

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GrammarError {
    /// The grammar contains a direct or indirect left-recursive cycle.
    LeftRecursion { cycle: Vec<String> },

    /// A branch can never be reached because an earlier branch in `one_of!`
    /// always succeeds or always commits before this branch is tried.
    UnreachableBranch { rule: String, branch_index: usize },

    /// Two or more profile guards on alternatives in `one_of!` are
    /// simultaneously active for the given profile.
    ConflictingGuards { rule: String },

    /// `GrammarBuilder::start()` was called but no start rule was registered.
    NoStartRule,

    /// A `ref_()` references a rule ID that was never registered.
    UnresolvedRef { rule_id: String },

    /// A rule was registered more than once.
    DuplicateRule { rule_id: String },
}
```

---

## RecoveryLandmarks

```rust
/// A set of token types that the recovery skip-forward logic recognizes as
/// safe re-entry points.
#[derive(Debug, Clone)]
pub struct RecoveryLandmarks {
    token_types: Vec<&'static str>,
}

impl RecoveryLandmarks {
    pub fn from_token_types(types: &[&'static str]) -> Self {
        RecoveryLandmarks { token_types: types.to_vec() }
    }

    pub fn contains(&self, token_type: &str) -> bool {
        self.token_types.iter().any(|t| *t == token_type)
    }
}
```

---

## Profile Guard Builder

A fluent DSL for constructing `RuleProfileGuard` values. Returned by the
free function `profile_guard()` (or `profile()` as an alias).

```rust
pub fn profile_guard() -> ProfileGuardBuilder;
pub fn profile() -> ProfileGuardBuilder; // alias

pub struct ProfileGuardBuilder { /* opaque */ }

impl ProfileGuardBuilder {
    /// Activate this rule starting from the given version string (e.g. `"2.0"`).
    pub fn since(self, version: &'static str) -> Self;

    /// Deactivate this rule at the given version string (exclusive upper bound).
    pub fn until(self, version: &'static str) -> Self;

    /// Require all named feature flags to be enabled.
    pub fn feature(self, flag: &'static str) -> Self;

    /// Require any of the given feature flags to be enabled.
    pub fn any_feature(self, flags: &[&'static str]) -> Self;

    /// Restrict to specific profile modes.
    pub fn mode(self, mode: ProfileMode) -> Self;

    /// Build the finished guard. Panics if no constraints were added
    /// (an unconstrained guard is always active, making `.enabled_if(...)` a no-op).
    pub fn build(self) -> RuleProfileGuard;
}
```

---

## Usage Examples

### JSON grammar

```rust
use rb_parser::prelude::*;

pub fn define_json_grammar() -> Grammar<JsonRule> {
    grammar()
        .rule(
            JsonRule::Member,
            node(
                json_syn::MEMBER,
                seq![
                    field("key", tok(json_tok::STRING)),
                    cut(),
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
                    node(json_syn::TRUE,   tok_sub(json_tok::LITERAL, "True")),
                    node(json_syn::FALSE,  tok_sub(json_tok::LITERAL, "False")),
                    node(json_syn::NULL,   tok_sub(json_tok::LITERAL, "Null")),
                ],
            ),
        )
        .start(JsonRule::Value)
}
```

### Profile guard on a rule

```rust
// Trailing commas are only allowed in v2+ or with the "trailing_commas" feature.
tok(json_tok::COMMA)
    .enabled_if(
        profile().since("2.0").build()
    )
```

### Recovery boundary on a rule

```rust
field(
    "members",
    list(ref_(JsonRule::Member), tok(json_tok::COMMA))
        .recover_to(any_of![json_tok::COMMA, json_tok::RBRACE]),
)
```

### Expression grammar with Pratt

```rust
.rule(
    ExprRule::Expr,
    pratt(ref_(ExprRule::Atom))
        .prefix(expr_tok::MINUS, 70, node(expr_syn::PREFIX))
        .infix_left(expr_tok::STAR,  60, node(expr_syn::BINARY))
        .infix_left(expr_tok::SLASH, 60, node(expr_syn::BINARY))
        .infix_left(expr_tok::PLUS,  50, node(expr_syn::BINARY))
        .infix_left(expr_tok::MINUS, 50, node(expr_syn::BINARY))
        .finish(),
)
```

---

## Implementation Notes

- `seq!` and `one_of!` expand to nested binary `seq2`/`alt2` calls. These
  internal helpers are not public; they are implementation details of the macros.
- `GrammarRule<R>` implementations are `'static` so they can be stored in the
  `Grammar<R>` map. Closures are not used in grammar rule bodies; all structure
  is expressed through the combinator primitives.
- `CompiledParser` is `Clone` and `Send + Sync`. It may be shared across
  threads once compiled; each parse session creates a fresh `ParseContext`.
- Binding power values for `pratt` are `u8` (0–255). By convention, the highest
  meaningful binding power should be `≤ 200` to leave headroom for future
  operators without renumbering.
