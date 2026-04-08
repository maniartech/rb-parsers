# Spec: Parser Consumption Surfaces

**Status**: Ready for implementation
**Module**: `rb_parser` (public surface); `rb_parser::events`, `rb_parser::visitors`
**Depends on**: `rb_parser::cst`, `rb_parser::engine`, `rb_common::diagnostics`
**Requirement source**: `docs/requirements/parser-execution-and-consumption-models.md`

---

## Decisions Made

| Question | Decision |
|---|---|
| Default public API | Tree-oriented. `CompiledParser::parse_tree()` is the primary entry point. |
| CST vs AST as default | CST first. AST is always a separate lowering step via `AstLoweringStrategy`. |
| Event surface | `CompiledParser::parse_events()` is an alternate output path using the same compiled grammar. |
| `IncrementalParser` type | Separate type with lifecycle state (cached tree, edit history). Justified by the different lifecycle: `CompiledParser` is immutable and `Send+Sync`; `IncrementalParser` carries mutable session state. |
| Strategy API (`parse_with_strategy`) | Available as `CompiledParser::parse_with_strategy()` for custom output types. Default surfaces do not require explicit strategy selection. |
| Visitor/walker location | Lives in `rb_parser::visitors` (a module inside `rb_parser`). Not a separate crate. |
| Trivia during traversal | Visitors receive a `visit_trivia_token` hook (separate from `visit_token`) so implementors can easily skip trivia without manual `is_trivia` checks. |
| Iterator surface | Convenience iterators (child iterator, descendant iterator, sibling walker) live on `CstTree` and `CstNode`. They are thin wrappers over the arena — no heap allocation. |
| Evaluators / semantic passes | Out of scope for `rb_parser`. Deferred to a future `rb_semantics` crate. |

---

## Module Layout

```
rb_parser
├── CompiledParser         (primary parse entry point)
├── IncrementalParser      (stateful incremental reuse)
├── TextEdit               (edit description for incremental parse)
│
rb_parser::events
├── ParseEvent
│
rb_parser::strategy
├── ParseStrategy          (trait)
├── CstBuildingStrategy    (built-in: produces CstTree)
├── EventCollectingStrategy (built-in: collects Vec<ParseEvent>)
│
rb_parser::visitors
├── TreeVisitor            (trait)
├── WalkOrder
└── DepthFirstWalker       (default DFS walker)
```

---

## ParseEvent

The push-model event stream emitted by the parse engine. Both `CstBuildingStrategy`
and `EventCollectingStrategy` consume this stream. Custom strategies may also
consume it directly.

```rust
use rb_common::spans::{SourceSpan, SourcePosition};
use rb_common::diagnostics::Diagnostic;
use rb_parser::cst::SyntaxKind;
use rb_parser::engine::RecoveryAction;

#[derive(Debug, Clone)]
pub enum ParseEvent {
    /// A syntax node boundary has opened. All events until the matching
    /// `NodeEnd` are children of this node.
    NodeStart {
        kind:       SyntaxKind,
        /// Byte offset of the first character included in the node.
        span_start: SourcePosition,
    },

    /// The previously opened node has closed.
    NodeEnd {
        kind: SyntaxKind,
        /// Full span of the closed node.
        span: SourceSpan,
    },

    /// A leaf token was consumed.
    Token {
        token_type:   &'static str,
        token_sub_kind: Option<&'static str>,
        span:         SourceSpan,
        /// `true` for whitespace, comments, and other trivia.
        is_trivia:    bool,
        /// Optional field name if this token was wrapped with `field(...)`.
        field_name:   Option<&'static str>,
    },

    /// A named field boundary was entered. Wraps one `NodeStart`/`NodeEnd`
    /// or one `Token`. The matching `FieldEnd` closes it.
    FieldStart { name: &'static str },
    FieldEnd   { name: &'static str },

    /// A diagnostic was emitted at this point in the stream.
    Error { diagnostic: Diagnostic },

    /// The engine applied an error-recovery action.
    Recovery { action: RecoveryAction },
}
```

---

## RecoveryAction

```rust
#[derive(Debug, Clone)]
pub enum RecoveryAction {
    /// The engine skipped `count` tokens to reach a recovery landmark.
    SkipTo { landmark_token_type: &'static str, skipped_count: usize },
    /// The engine inserted a synthetic token to satisfy a required terminal.
    InsertSynthetic { token_type: &'static str, at: SourcePosition },
    /// Recovery was exhausted; parsing halted at this position.
    Halted { at: SourcePosition },
}
```

---

## ParseStrategy (trait)

```rust
/// A push-model consumer of `ParseEvent`s.
///
/// Built-in strategies are `CstBuildingStrategy` and `EventCollectingStrategy`.
/// Custom strategies allow building fully custom output types without modifying
/// the grammar.
pub trait ParseStrategy: Sized {
    /// The output type produced by this strategy.
    type Output;

    /// Called for each `ParseEvent` in emission order.
    fn on_event(&mut self, event: ParseEvent);

    /// Called once after the last event. Consumes `self` and returns output.
    fn finish(self) -> Self::Output;
}
```

### CstBuildingStrategy

```rust
/// Builds a `CstTree` from the event stream. This is the strategy used by
/// `CompiledParser::parse_tree()`.
pub struct CstBuildingStrategy {
    // opaque internal node/token arenas being built
}

impl ParseStrategy for CstBuildingStrategy {
    type Output = CstTree;
    fn on_event(&mut self, event: ParseEvent) { /* ... */ }
    fn finish(self) -> CstTree { /* ... */ }
}
```

### EventCollectingStrategy

```rust
/// Collects raw `ParseEvent`s into a `Vec`. Useful for streaming consumers,
/// testing, and lower-allocation validation workflows.
pub struct EventCollectingStrategy {
    events: Vec<ParseEvent>,
}

impl EventCollectingStrategy {
    pub fn new() -> Self { EventCollectingStrategy { events: Vec::new() } }
}

impl ParseStrategy for EventCollectingStrategy {
    type Output = Vec<ParseEvent>;
    fn on_event(&mut self, event: ParseEvent) { self.events.push(event); }
    fn finish(self) -> Vec<ParseEvent> { self.events }
}
```

---

## CompiledParser

The primary parse API. Immutable after construction. `Send + Sync`.

```rust
use rb_common::diagnostics::DiagnosticsContext;
use rb_tokenizer::tokens::TokenStream;

pub struct CompiledParser { /* opaque; wraps compiled grammar IR + profile */ }

impl CompiledParser {
    // ──────────── Primary surfaces ─────────────────────────────────

    /// Parse the token stream and produce a `CstTree`.
    /// Diagnostics are emitted into `ctx`.
    /// This is the default and recommended entry point.
    pub fn parse_tree(
        &self,
        stream: &TokenStream<'_>,
        ctx:    &mut DiagnosticsContext,
    ) -> CstTree;

    /// Parse the token stream and return a flat `Vec<ParseEvent>`.
    /// More memory-efficient than `parse_tree` when the caller only needs to
    /// inspect or stream events without building a tree.
    pub fn parse_events(
        &self,
        stream: &TokenStream<'_>,
        ctx:    &mut DiagnosticsContext,
    ) -> Vec<ParseEvent>;

    // ──────────── Strategy surface ─────────────────────────────────

    /// Parse using a custom `ParseStrategy`. The grammar is the same;
    /// the output type is determined by the strategy.
    pub fn parse_with_strategy<S: ParseStrategy>(
        &self,
        stream:   &TokenStream<'_>,
        ctx:      &mut DiagnosticsContext,
        strategy: S,
    ) -> S::Output;

    // ──────────── Incremental surface ──────────────────────────────

    /// Create a stateful `IncrementalParser` from this compiled grammar.
    /// The `CompiledParser` must outlive the `IncrementalParser`.
    pub fn incremental(&self) -> IncrementalParser<'_>;
}
```

---

## IncrementalParser

A stateful parse session that caches the most recently produced `CstTree`
and attempts to reuse unchanged subtrees across edits.

```rust
pub struct IncrementalParser<'compiled> {
    compiled:    &'compiled CompiledParser,
    cached_tree: Option<CstTree>,
}

impl<'compiled> IncrementalParser<'compiled> {
    /// Perform the initial full parse and cache the result.
    pub fn initial_parse(
        &mut self,
        stream: &TokenStream<'_>,
        ctx:    &mut DiagnosticsContext,
    ) -> &CstTree;

    /// Re-parse after one or more edits to the source.
    ///
    /// `edits` must be non-overlapping and sorted by start offset.
    /// Subtrees that are entirely outside all edited regions are eligible
    /// for reuse from the cached tree. Whether a subtree is actually reused
    /// is an implementation detail; correctness must be guaranteed regardless.
    pub fn reparse(
        &mut self,
        stream: &TokenStream<'_>,
        edits:  &[TextEdit],
        ctx:    &mut DiagnosticsContext,
    ) -> &CstTree;

    /// Discard the cached tree and force a full re-parse on the next call.
    pub fn invalidate(&mut self);

    /// Returns a reference to the most recently produced tree, or `None` if
    /// no parse has been performed yet.
    pub fn cached_tree(&self) -> Option<&CstTree>;
}
```

---

## TextEdit

```rust
/// A single non-overlapping source edit, given as a byte-range replacement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEdit {
    /// The byte range being replaced (half-open: `start..end`).
    pub range: std::ops::Range<usize>,
    /// The replacement text. Empty string means deletion.
    pub replacement: String,
}

impl TextEdit {
    pub fn insert(at: usize, text: impl Into<String>) -> Self {
        TextEdit { range: at..at, replacement: text.into() }
    }

    pub fn delete(range: std::ops::Range<usize>) -> Self {
        TextEdit { range, replacement: String::new() }
    }

    pub fn replace(
        range: std::ops::Range<usize>,
        text: impl Into<String>,
    ) -> Self {
        TextEdit { range, replacement: text.into() }
    }
}
```

---

## TreeVisitor (trait)

Traversal callback interface over a `CstTree`. Implement this to perform
post-parse tree inspection without exposing traversal internals.

```rust
pub trait TreeVisitor {
    /// Called when entering a `CstNode`, before visiting its children.
    fn visit_node_enter(&mut self, _node: &CstNode, _tree: &CstTree) {}

    /// Called when leaving a `CstNode`, after all children have been visited.
    fn visit_node_exit(&mut self, _node: &CstNode, _tree: &CstTree) {}

    /// Called for each semantic (non-trivia) token.
    fn visit_token(&mut self, _token: &CstToken, _tree: &CstTree) {}

    /// Called for each trivia token (whitespace, comments).
    /// Default implementation does nothing, so visitors that do not care
    /// about trivia have zero overhead from trivia in the tree.
    fn visit_trivia_token(&mut self, _token: &CstToken, _tree: &CstTree) {}
}
```

---

## WalkOrder

```rust
/// Controls the order in which `TreeVisitor` callbacks arrive relative to
/// a node's children.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WalkOrder {
    /// `visit_node_enter` before children; `visit_node_exit` after children.
    DepthFirst,
    /// Not supported in Phase 1. Reserved for future breadth-first traversal.
    BreadthFirst,
}
```

---

## CstTree Traversal API

These methods are defined on `CstTree` (see `rb_parser-cst-layout.md`).

```rust
impl CstTree {
    /// Walk the entire tree depth-first, calling `visitor` callbacks.
    pub fn walk(&self, visitor: &mut dyn TreeVisitor, order: WalkOrder);

    /// Walk the subtree rooted at `node_id` depth-first.
    pub fn walk_node(
        &self,
        node_id:  SyntaxNodeId,
        visitor:  &mut dyn TreeVisitor,
        order:    WalkOrder,
    );

    /// Iterate over all direct child nodes of `node_id`.
    pub fn children_of(
        &self,
        node_id: SyntaxNodeId,
    ) -> impl Iterator<Item = &CstNode> + '_;

    /// Iterate over all direct child tokens of `node_id`, skipping trivia.
    pub fn tokens_of(
        &self,
        node_id: SyntaxNodeId,
    ) -> impl Iterator<Item = &CstToken> + '_;

    /// Iterate over all descendants (nodes only) of `node_id` in pre-order.
    pub fn descendants(
        &self,
        node_id: SyntaxNodeId,
    ) -> impl Iterator<Item = &CstNode> + '_;

    /// All nodes whose kind matches `kind`.
    pub fn nodes_of_kind(
        &self,
        kind: SyntaxKind,
    ) -> impl Iterator<Item = &CstNode> + '_;
}
```

---

## Example: Tree walk

```rust
use rb_parser::visitors::{TreeVisitor, WalkOrder};
use rb_parser::cst::{CstNode, CstToken, CstTree};

struct CountNodes { count: usize }

impl TreeVisitor for CountNodes {
    fn visit_node_enter(&mut self, _node: &CstNode, _tree: &CstTree) {
        self.count += 1;
    }
}

let tree: CstTree = parser.parse_tree(&tokens, &mut ctx);
let mut counter = CountNodes { count: 0 };
tree.walk(&mut counter, WalkOrder::DepthFirst);
println!("total nodes: {}", counter.count);
```

---

## Example: Incremental parse

```rust
let parser: CompiledParser = grammar.compile(&profile)?;
let mut incremental = parser.incremental();

// Initial full parse
let tree = incremental.initial_parse(&tokens_v1, &mut ctx);

// After an edit: replace bytes 10..15 with "true"
let edits = vec![TextEdit::replace(10..15, "true")];
let updated_tree = incremental.reparse(&tokens_v2, &edits, &mut ctx);
```

---

## Example: Custom strategy

```rust
use rb_parser::strategy::ParseStrategy;
use rb_parser::events::ParseEvent;

struct TokenCounter { semantic: usize, trivia: usize }

impl ParseStrategy for TokenCounter {
    type Output = (usize, usize);

    fn on_event(&mut self, event: ParseEvent) {
        if let ParseEvent::Token { is_trivia, .. } = event {
            if is_trivia { self.trivia += 1; } else { self.semantic += 1; }
        }
    }

    fn finish(self) -> (usize, usize) {
        (self.semantic, self.trivia)
    }
}

let (semantic, trivia) = parser.parse_with_strategy(&tokens, &mut ctx, TokenCounter { semantic: 0, trivia: 0 });
```

---

## Implementation Notes

- `CompiledParser::parse_tree()` is equivalent to
  `parse_with_strategy(&tokens, &mut ctx, CstBuildingStrategy::new())`.
  The default method exists to avoid strategy boilerplate in common usage.
- `IncrementalParser` holds a `&'compiled CompiledParser` borrow. This means
  the compiled grammar must not be dropped while the incremental session is
  alive. This lifetime is expressed at the type level.
- The incremental reuse algorithm in Phase 1 may be a simple "reuse nothing,
  full reparse" implementation. The public API contract must be correct even
  if Phase 1 reuse is conservative. The `TextEdit` API guarantees the contract
  for callers regardless of the reuse strategy.
- `BreadthFirst` walk order is reserved but not implemented in Phase 1.
  Callers who request it will receive an `unimplemented!()` panic in debug
  builds and a depth-first walk in release builds until Phase 2.
