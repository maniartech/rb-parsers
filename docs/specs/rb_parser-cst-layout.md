# Spec: CST Layout and Syntax Tree

**Status**: Ready for implementation
**Module**: `rb_parser::cst`
**Depends on**: `rb_common::spans` (`SourceSpan`, `SourceId`)
**Requirement source**: `docs/requirements/syntax-tree-and-materialization.md`

---

## Decisions Made

| Question | Decision |
|---|---|
| Tree representation | Arena/index-based. Nodes and tokens are stored in flat `Vec`s inside `CstTree`; references use `u32` indices. No `Rc`, no `Arc`, no pointer chasing. |
| Trivia (whitespace, comments) | Explicit trivia tokens stored as `CstToken { is_trivia: true }`. Trivia is retained in the tree for source-faithful round-tripping but can be skipped during semantic traversal. |
| AST materialization | Separate lowering step via `AstLoweringStrategy`. The CST is the canonical parse product; AST is always a derived view. |
| `SyntaxKind` identity | `&'static str` wrapping — no global integer registry needed. Equality is string equality so language authors do not need to coordinate integer allocations. |
| `NodeOrToken` union | Children of a node are stored as `Vec<NodeOrToken>` mapping indices into the same arena. This keeps child iteration cheap and allocation bounded. |
| Source text | **Not stored in the CST.** Byte offsets in `SourceSpan` are stable indices into the original source buffer. Source text access goes through `SourceStore`. |
| Field access | Nodes tagged with `field(name, ...)` in the grammar store field names alongside `NodeOrToken` children so callers can look up children by name. |

---

## Module Layout

```
rb_parser::cst
├── SyntaxKind
├── SyntaxNodeId
├── SyntaxTokenId
├── NodeOrToken
├── CstNodeChild      (NodeOrToken + optional field name)
├── CstNode
├── CstToken
├── CstTree
├── MaterializationStrategy  (trait)
└── AstLoweringStrategy      (trait)
```

---

## Types

### SyntaxKind

A lightweight wrapper over a `&'static str`. Equality is string equality so
two `SyntaxKind` values defined in different crates compare equal when their
strings are identical. This is intentional: identity is by name, not by address.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SyntaxKind(pub &'static str);

impl SyntaxKind {
    pub const fn new(s: &'static str) -> Self {
        SyntaxKind(s)
    }

    pub fn as_str(self) -> &'static str {
        self.0
    }
}

impl std::fmt::Display for SyntaxKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}
```

---

### SyntaxNodeId / SyntaxTokenId

Typed, compact indices into the `CstTree` arenas. `u32` is deliberately chosen:
it supports trees with up to 4 billion nodes, which is far beyond any realistic
source file, while keeping each child entry to 4 bytes.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SyntaxNodeId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SyntaxTokenId(pub u32);
```

---

### NodeOrToken

The discriminated union used in child lists. Variants are `u8`-tagged so the
enum fits in 8 bytes on most platforms.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeOrToken {
    Node(SyntaxNodeId),
    Token(SyntaxTokenId),
}

impl NodeOrToken {
    pub fn as_node(self) -> Option<SyntaxNodeId> {
        match self { NodeOrToken::Node(id) => Some(id), _ => None }
    }

    pub fn as_token(self) -> Option<SyntaxTokenId> {
        match self { NodeOrToken::Token(id) => Some(id), _ => None }
    }
}
```

---

### CstNodeChild

One entry in a node's child list. Carries an optional field name assigned by
a `field(name, rule)` combinator. The name is `None` for unlabeled children.

```rust
#[derive(Debug, Clone)]
pub struct CstNodeChild {
    /// The child node or token.
    pub child: NodeOrToken,
    /// Optional field label from `field("name", rule)`.
    pub field_name: Option<&'static str>,
}
```

---

### CstNode

One interior node in the syntax tree. A node groups zero or more children
under a named syntactic category.

```rust
#[derive(Debug, Clone)]
pub struct CstNode {
    /// Stable index of this node in the owning `CstTree`.
    pub id: SyntaxNodeId,
    /// The syntactic category (e.g. `SyntaxKind::new("JsonObject")`).
    pub kind: SyntaxKind,
    /// Source span covering this node, inclusive of all its children's spans.
    pub span: SourceSpan,
    /// Ordered children. Includes both named fields and unlabeled children.
    pub children: Vec<CstNodeChild>,
}

impl CstNode {
    /// Returns the first child with a matching field name, or `None`.
    pub fn field(&self, name: &str) -> Option<NodeOrToken> {
        self.children
            .iter()
            .find(|c| c.field_name == Some(name))
            .map(|c| c.child)
    }

    /// Returns all non-trivia token children directly under this node.
    pub fn direct_tokens<'a>(
        &'a self,
    ) -> impl Iterator<Item = SyntaxTokenId> + 'a {
        self.children.iter().filter_map(|c| c.child.as_token())
    }

    /// Returns all direct child node IDs.
    pub fn direct_nodes<'a>(
        &'a self,
    ) -> impl Iterator<Item = SyntaxNodeId> + 'a {
        self.children.iter().filter_map(|c| c.child.as_node())
    }
}
```

---

### CstToken

One leaf token in the syntax tree.

```rust
#[derive(Debug, Clone)]
pub struct CstToken {
    /// Stable index of this token in the owning `CstTree`.
    pub id: SyntaxTokenId,
    /// The token type string matching `TokenKind::kind` from `rb_tokenizer`.
    pub token_type: &'static str,
    /// The token's optional sub-kind, matching `TokenKind::sub_kind`.
    pub token_sub_kind: Option<&'static str>,
    /// Byte-precise source span of this token.
    pub span: SourceSpan,
    /// `true` for whitespace, comments, and other trivia that should be
    /// skipped during semantic tree traversal.
    pub is_trivia: bool,
}

impl CstToken {
    /// Returns `true` when this token carries semantic meaning (not trivia).
    pub fn is_semantic(&self) -> bool {
        !self.is_trivia
    }
}
```

---

### CstTree

The complete immutable syntax tree produced by a parse. Owns all node and
token arenas.

```rust
use rb_common::spans::SourceId;

pub struct CstTree {
    nodes:  Vec<CstNode>,
    tokens: Vec<CstToken>,
    root:   SyntaxNodeId,
    source_id: SourceId,
}

impl CstTree {
    // ──────────────────────── Root access ────────────────────────────

    /// The root node of the tree.
    pub fn root(&self) -> &CstNode {
        self.node(self.root)
    }

    pub fn root_id(&self) -> SyntaxNodeId {
        self.root
    }

    // ──────────────────────── Direct access ──────────────────────────

    /// Panics if `id` is out of range. Prefer this over `get_node` in code
    /// that holds a tree-issued `SyntaxNodeId` (they are always valid within
    /// their owning tree).
    pub fn node(&self, id: SyntaxNodeId) -> &CstNode {
        &self.nodes[id.0 as usize]
    }

    pub fn token(&self, id: SyntaxTokenId) -> &CstToken {
        &self.tokens[id.0 as usize]
    }

    // ──────────────────────── Convenience helpers ─────────────────────

    /// Resolves a `NodeOrToken` to a human-readable kind string for debugging.
    pub fn kind_of(&self, child: NodeOrToken) -> &str {
        match child {
            NodeOrToken::Node(id)  => self.node(id).kind.as_str(),
            NodeOrToken::Token(id) => self.token(id).token_type,
        }
    }

    /// Returns the span of any `NodeOrToken`.
    pub fn span_of(&self, child: NodeOrToken) -> SourceSpan {
        match child {
            NodeOrToken::Node(id)  => self.node(id).span,
            NodeOrToken::Token(id) => self.token(id).span,
        }
    }

    /// Looks up a named field child of `node_id` and resolves it to a node,
    /// or `None` if the field is absent or resolves to a token.
    pub fn field_node(&self, node_id: SyntaxNodeId, name: &str) -> Option<&CstNode> {
        let child = self.node(node_id).field(name)?;
        child.as_node().map(|id| self.node(id))
    }

    /// Looks up a named field child of `node_id` and resolves it to a token,
    /// or `None` if the field is absent or resolves to a node.
    pub fn field_token(&self, node_id: SyntaxNodeId, name: &str) -> Option<&CstToken> {
        let child = self.node(node_id).field(name)?;
        child.as_token().map(|id| self.token(id))
    }

    /// All nodes in the tree whose kind matches `kind`.
    pub fn nodes_of_kind(&self, kind: SyntaxKind) -> impl Iterator<Item = &CstNode> {
        self.nodes.iter().filter(move |n| n.kind == kind)
    }

    /// The `SourceId` this tree was built from.
    pub fn source_id(&self) -> SourceId {
        self.source_id
    }

    // ──────────────────────── Trivia helpers ─────────────────────────

    /// Iterator over all semantic (non-trivia) tokens in source order.
    pub fn semantic_tokens(&self) -> impl Iterator<Item = &CstToken> {
        self.tokens.iter().filter(|t| !t.is_trivia)
    }
}
```

---

### MaterializationStrategy (trait)

Allows the parse engine to drive multiple output types through a common push
interface. The parser emits `ParseEvent`s; the strategy converts them to the
desired output form. See `rb_parser-consumption-surfaces.md` for `ParseEvent`.

```rust
/// Consumes parse events and produces a typed output.
///
/// Built-in strategies ship with `rb_parser`:
/// - `CstBuildingStrategy` — produces a `CstTree`
/// - `EventCollectingStrategy` — collects a `Vec<ParseEvent>`
///
/// Custom strategies must implement this trait.
pub trait MaterializationStrategy: Sized {
    type Output;

    /// Called for each parse event in emission order.
    fn on_event(&mut self, event: crate::events::ParseEvent);

    /// Called once after all events have been emitted.
    /// Consumes `self` and returns the finished output.
    fn finish(self) -> Self::Output;
}
```

---

### AstLoweringStrategy (trait)

A post-parse transformation from `CstTree` to a typed AST. This is always a
separate step: the grammar produces a `CstTree`, and a language-specific
lowering pass constructs AST nodes.

```rust
pub trait AstLoweringStrategy {
    /// The language-specific AST type.
    type Output;
    /// The error type for invalid or unexpected CST structure.
    type Error;

    /// Consume the CST and produce an AST. Called after parsing is complete.
    fn lower(&self, tree: &CstTree) -> Result<Self::Output, Self::Error>;
}
```

---

## Trivia Model

Trivia (whitespace, comments, blank lines) is stored as `CstToken { is_trivia: true }`.

**Rationale**: This approach preserves full source fidelity (the tree can
reconstruct the exact source text), avoids a separate trivia attachment phase,
and keeps the tree homogeneous. The cost is that semantic traversal must skip
trivia tokens — the `semantic_tokens()` iterator and the visitor's
`visit_trivia_token` hook handle this transparently.

**Trivia assignment rule**: During CST construction, trivia tokens are attached
as children of the **innermost node** that spans them. Trivia between two
sibling nodes falls to the left sibling's parent if it precedes the right
sibling. This rule is deterministic and requires no heuristic lookahead.

**Trivia and round-tripping**: The concatenation of all `CstToken::span` byte
ranges in tree order must equal the original source buffer. Implementations
must uphold this invariant.

---

## Memory and Performance Notes

- A typical source file with 10 000 tokens occupies roughly 800 KB of CST
  arena (`~80 bytes` per node/token entry for all fields). This is acceptable
  for parse-and-inspect workflows.
- For streaming or validation-only workflows where the CST is not needed, use
  `EventCollectingStrategy` to avoid tree allocation entirely.
- `CstTree` is `!Send` until explicitly wrapped. Callers that need to pass a
  tree across threads must clone it or wrap it in `Arc`.

---

## Usage Example

```rust
use rb_parser::cst::{CstTree, SyntaxKind};

// After parsing…
let tree: CstTree = parser.parse_tree(&token_stream, &mut ctx);

let root = tree.root();
println!("root kind: {}", root.kind);

// Access a named field
if let Some(key_token) = tree.field_token(root.id, "key") {
    println!("key span: {:?}", key_token.span);
}

// Find all nodes of a given kind
for member in tree.nodes_of_kind(SyntaxKind::new("JsonMember")) {
    println!("member @ {:?}", member.span);
}
```
