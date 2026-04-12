use rb_common::spans::{SourceId, SourceSpan};

// ── SyntaxKind ────────────────────────────────────────────────────────────────

/// A lightweight syntactic category tag. Identity is by string value so language
/// authors in different crates do not need to co-ordinate integer allocations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
// &'static str cannot implement Deserialize since 'de does not satisfy 'static;
// Serialize only for serialization to cache / disk.
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct SyntaxKind(pub &'static str);

impl SyntaxKind {
    /// Constructs a `SyntaxKind` from a `'static` string slice.
    pub const fn new(s: &'static str) -> Self {
        SyntaxKind(s)
    }

    /// Returns the underlying string tag.
    pub fn as_str(self) -> &'static str {
        self.0
    }
}

impl std::fmt::Display for SyntaxKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

// ── Compact arena indices ─────────────────────────────────────────────────────

/// Typed index into the `CstTree::nodes` arena.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SyntaxNodeId(pub u32);

/// Typed index into the `CstTree::tokens` arena.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SyntaxTokenId(pub u32);

// ── NodeOrToken ───────────────────────────────────────────────────────────────

/// Discriminated union of a node reference or a token reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum NodeOrToken {
    /// The child is an interior syntax node.
    Node(SyntaxNodeId),
    /// The child is a leaf token.
    Token(SyntaxTokenId),
}

impl NodeOrToken {
    /// Returns `Some(id)` if this is a [`Node`](NodeOrToken::Node) variant.
    pub fn as_node(self) -> Option<SyntaxNodeId> {
        match self { NodeOrToken::Node(id) => Some(id), _ => None }
    }

    /// Returns `Some(id)` if this is a [`Token`](NodeOrToken::Token) variant.
    pub fn as_token(self) -> Option<SyntaxTokenId> {
        match self { NodeOrToken::Token(id) => Some(id), _ => None }
    }
}

// ── CstNodeChild ─────────────────────────────────────────────────────────────

/// One entry in a node's child list. Carries an optional field label.
#[derive(Debug, Clone)]
// Contains SyntaxKind (&'static str) — serialize only.
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct CstNodeChild {
    /// The referenced child node or leaf token.
    pub child: NodeOrToken,
    /// Optional field label from a `field("name", rule)` combinator.
    pub field_name: Option<&'static str>,
}

// ── CstNode ───────────────────────────────────────────────────────────────────

/// An interior node in the concrete syntax tree.
#[derive(Debug, Clone)]
// Contains SyntaxKind (&'static str) — serialize only.
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct CstNode {
    /// Unique identity of this node within its `CstTree`.
    pub id: SyntaxNodeId,
    /// Syntactic category of this node.
    pub kind: SyntaxKind,
    /// Source range covered by this node and all its descendants.
    pub span: SourceSpan,
    /// Ordered list of child nodes and leaf tokens.
    pub children: Vec<CstNodeChild>,
    /// `true` when this node was synthesised by the error-recovery strategy to
    /// represent an unclosed or missing construct. Such nodes may have a
    /// zero-width span that is otherwise indistinguishable from a genuinely
    /// zero-width production.
    pub is_error_recovery: bool,
}

impl CstNode {
    /// Returns the first child with the given field name.
    pub fn field(&self, name: &str) -> Option<NodeOrToken> {
        self.children
            .iter()
            .find(|c| c.field_name == Some(name))
            .map(|c| c.child)
    }

    /// Returns the IDs of **all** token children of this node — both semantic
    /// tokens and trivia (whitespace, comments, etc.).
    ///
    /// > **Note:** This is inconsistent with [`CstTree::tokens_of`] which
    /// > only returns semantic (non-trivia) tokens. Prefer
    /// > [`CstNode::direct_semantic_tokens`] when you want the same behaviour.
    pub fn direct_tokens(&self) -> impl Iterator<Item = SyntaxTokenId> + '_ {
        self.children.iter().filter_map(|c| c.child.as_token())
    }

    /// Returns the IDs of **semantic** (non-trivia) token children of this
    /// node. Consistent with [`CstTree::tokens_of`].
    ///
    /// Use [`direct_tokens`](Self::direct_tokens) if you also need trivia tokens.
    pub fn direct_semantic_tokens<'a>(
        &'a self,
        tree: &'a CstTree,
    ) -> impl Iterator<Item = SyntaxTokenId> + 'a {
        self.children
            .iter()
            .filter_map(|c| c.child.as_token())
            .filter(move |&id| !tree.token(id).is_trivia)
    }

    /// Returns an iterator over the IDs of all direct child *nodes* (no tokens).
    pub fn direct_nodes(&self) -> impl Iterator<Item = SyntaxNodeId> + '_ {
        self.children.iter().filter_map(|c| c.child.as_node())
    }
}

// ── CstToken ──────────────────────────────────────────────────────────────────

/// A leaf token in the concrete syntax tree.
#[derive(Debug, Clone)]
// Contains &'static str fields — serialize only.
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct CstToken {
    /// Unique identity of this token within its `CstTree`.
    pub id: SyntaxTokenId,
    /// The scanner-assigned token-type name (e.g., `"STRING"`).
    pub token_type: &'static str,
    /// Optional sub-kind set by `tok_sub` combinators (e.g., operator flavour).
    pub token_sub_kind: Option<&'static str>,
    /// Source range of this token.
    pub span: SourceSpan,
    /// `true` for whitespace, comments, and other trivia.
    pub is_trivia: bool,
}

impl CstToken {
    /// Returns `true` for non-trivia tokens; the inverse of `is_trivia`.
    pub fn is_semantic(&self) -> bool {
        !self.is_trivia
    }

    /// Returns the source text slice that this token covers.
    ///
    /// # Panics
    /// Panics if the token's byte-offset range is out of bounds for `source`.
    /// The `source` string must be the same input that was parsed to produce this tree.
    #[inline]
    pub fn text<'a>(&self, source: &'a str) -> &'a str {
        let start = self.span.start.byte_offset;
        let end   = self.span.end.byte_offset;
        &source[start..end]
    }
}

// ── CstTree ───────────────────────────────────────────────────────────────────

/// The complete immutable concrete syntax tree produced by a parse.
pub struct CstTree {
    nodes: Vec<CstNode>,
    tokens: Vec<CstToken>,
    root: SyntaxNodeId,
    source_id: SourceId,
}

impl CstTree {
    /// Construct a tree from pre-built arenas. Called by the tree-building strategy.
    pub fn new(
        nodes: Vec<CstNode>,
        tokens: Vec<CstToken>,
        root: SyntaxNodeId,
        source_id: SourceId,
    ) -> Self {
        CstTree { nodes, tokens, root, source_id }
    }

    // ── Root access ──────────────────────────────────────────────────────────

    /// Returns a reference to the root node.
    pub fn root(&self) -> &CstNode {
        self.node(self.root)
    }

    /// Returns the arena ID of the root node.
    pub fn root_id(&self) -> SyntaxNodeId {
        self.root
    }

    // ── Direct arena access ───────────────────────────────────────────────────

    /// Returns the node at `id`.
    pub fn node(&self, id: SyntaxNodeId) -> &CstNode {
        &self.nodes[id.0 as usize]
    }

    /// Returns the token at `id`.
    pub fn token(&self, id: SyntaxTokenId) -> &CstToken {
        &self.tokens[id.0 as usize]
    }

    // ── Convenience helpers ───────────────────────────────────────────────────

    /// Returns the kind or token-type string for any `NodeOrToken`.
    pub fn kind_of(&self, child: NodeOrToken) -> &str {
        match child {
            NodeOrToken::Node(id)  => self.node(id).kind.as_str(),
            NodeOrToken::Token(id) => self.token(id).token_type,
        }
    }

    /// Returns the source span of a `NodeOrToken`.
    pub fn span_of(&self, child: NodeOrToken) -> SourceSpan {
        match child {
            NodeOrToken::Node(id)  => self.node(id).span,
            NodeOrToken::Token(id) => self.token(id).span,
        }
    }

    /// Returns the named-field child *node* of `node_id`, if present.
    pub fn field_node(&self, node_id: SyntaxNodeId, name: &str) -> Option<&CstNode> {
        let child = self.node(node_id).field(name)?;
        child.as_node().map(|id| self.node(id))
    }

    /// Returns the named-field child *token* of `node_id`, if present.
    pub fn field_token(&self, node_id: SyntaxNodeId, name: &str) -> Option<&CstToken> {
        let child = self.node(node_id).field(name)?;
        child.as_token().map(|id| self.token(id))
    }

    /// Returns an iterator over all nodes in the tree that have the given `kind`.
    pub fn nodes_of_kind(&self, kind: SyntaxKind) -> impl Iterator<Item = &CstNode> {
        self.nodes.iter().filter(move |n| n.kind == kind)
    }

    /// Returns the `SourceId` this tree was parsed from.
    pub fn source_id(&self) -> SourceId {
        self.source_id
    }

    /// Return the total number of nodes in the tree.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Iterate over all node IDs in the tree.
    pub fn node_ids(&self) -> impl Iterator<Item = SyntaxNodeId> {
        (0..self.nodes.len()).map(|i| SyntaxNodeId(i as u32))
    }

    // ── Trivia helpers ────────────────────────────────────────────────────────

    /// Returns all non-trivia tokens in the tree, in source order.
    pub fn semantic_tokens(&self) -> impl Iterator<Item = &CstToken> {
        self.tokens.iter().filter(|t| !t.is_trivia)
    }

    // ── Traversal helpers (used by visitors) ─────────────────────────────────

    /// Returns the direct child *nodes* of `node_id`.
    pub fn children_of(&self, node_id: SyntaxNodeId) -> impl Iterator<Item = &CstNode> + '_ {
        self.node(node_id)
            .children
            .iter()
            .filter_map(|c| c.child.as_node().map(|id| self.node(id)))
    }

    /// Returns the direct non-trivia tokens of `node_id`.
    pub fn tokens_of(&self, node_id: SyntaxNodeId) -> impl Iterator<Item = &CstToken> + '_ {
        self.node(node_id)
            .children
            .iter()
            .filter_map(|c| c.child.as_token().map(|id| self.token(id)))
            .filter(|t| !t.is_trivia)
    }

    /// Returns all descendant nodes of `node_id` in pre-order, excluding the node itself.
    pub fn descendants(&self, node_id: SyntaxNodeId) -> impl Iterator<Item = &CstNode> + '_ {
        // Pre-order DFS using an explicit stack to avoid recursion.
        let mut stack: Vec<SyntaxNodeId> = self
            .node(node_id)
            .children
            .iter()
            .filter_map(|c| c.child.as_node())
            .collect();
        stack.reverse();
        std::iter::from_fn(move || {
            let id = stack.pop()?;
            let node = &self.nodes[id.0 as usize];
            let mut children: Vec<SyntaxNodeId> = node
                .children
                .iter()
                .filter_map(|c| c.child.as_node())
                .collect();
            children.reverse();
            stack.extend(children);
            Some(node)
        })
    }

    /// Drives `visitor` in depth-first order over the whole tree.
    pub fn walk(&self, visitor: &mut dyn crate::visitors::TreeVisitor) {
        self.walk_node(self.root, visitor);
    }

    /// Drives `visitor` in depth-first order over the subtree rooted at `node_id`.
    pub fn walk_node(&self, node_id: SyntaxNodeId, visitor: &mut dyn crate::visitors::TreeVisitor) {
        // Collect the lightweight `NodeOrToken` IDs (Copy) before any visitor call so
        // that the borrow of `self` is not held across mutable visitor callbacks.
        let children: Vec<NodeOrToken> = self.node(node_id)
            .children
            .iter()
            .map(|c| c.child)
            .collect();

        visitor.visit_node_enter(self.node(node_id), self);
        for child in children {
            match child {
                NodeOrToken::Node(id) => self.walk_node(id, visitor),
                NodeOrToken::Token(id) => {
                    let tok = self.token(id);
                    if tok.is_trivia {
                        visitor.visit_trivia_token(tok, self);
                    } else {
                        visitor.visit_token(tok, self);
                    }
                }
            }
        }
        visitor.visit_node_exit(self.node(node_id), self);
    }

    // ── Debug / test output ────────────────────────────────────────────────────

    /// Returns an s-expression representation of the tree rooted at `node_id`.
    ///
    /// If `source` is `Some`, leaf tokens include their source text; otherwise
    /// only the token type is shown.
    ///
    /// Example output:
    /// ```text
    /// (Root
    ///   (Object
    ///     (Pair
    ///       "key"@String
    ///       ":"@Colon
    ///       "42"@Number)))
    /// ```
    pub fn to_sexpr_from(&self, node_id: SyntaxNodeId, source: Option<&str>) -> String {
        let mut out = String::new();
        self.sexpr_node(node_id, source, 0, &mut out);
        out
    }

    /// Convenience wrapper: s-expression for the entire tree.
    pub fn to_sexpr(&self, source: Option<&str>) -> String {
        self.to_sexpr_from(self.root, source)
    }

    fn sexpr_node(&self, node_id: SyntaxNodeId, source: Option<&str>, depth: usize, out: &mut String) {
        let node = self.node(node_id);
        let indent = "  ".repeat(depth);
        out.push_str(&indent);
        out.push('(');
        out.push_str(node.kind.as_str());
        // Filter out trivia for the s-expression (it clutters the output)
        let semantic_children: Vec<&CstNodeChild> = node.children
            .iter()
            .filter(|c| match c.child {
                NodeOrToken::Token(id) => !self.token(id).is_trivia,
                NodeOrToken::Node(_)   => true,
            })
            .collect();
        for child_entry in semantic_children {
            out.push('\n');
            match child_entry.child {
                NodeOrToken::Node(id) => {
                    self.sexpr_node(id, source, depth + 1, out);
                }
                NodeOrToken::Token(id) => {
                    let tok = self.token(id);
                    let child_indent = "  ".repeat(depth + 1);
                    out.push_str(&child_indent);
                    if let Some(field) = child_entry.field_name {
                        out.push_str(field);
                        out.push(':');
                    }
                    if let Some(src) = source {
                        let text = tok.text(src);
                        out.push('"');
                        out.push_str(&text.replace('\\', "\\\\").replace('"', "\\\""));
                        out.push('"');
                        out.push('@');
                    }
                    out.push_str(tok.token_type);
                }
            }
        }
        out.push(')')
    }
}

// ── Lowering trait ────────────────────────────────────────────────────────────

/// A post-parse transformation from `CstTree` to a typed AST.
pub trait AstLoweringStrategy {
    /// The resulting AST type produced by [`lower`](Self::lower).
    type Output;
    /// The error type returned when lowering fails.
    type Error;
    /// Transforms `tree` into an instance of `Self::Output`.
    fn lower(&self, tree: &CstTree) -> Result<Self::Output, Self::Error>;
}
