use rb_common::spans::{SourceId, SourceSpan};

// ── SyntaxKind ────────────────────────────────────────────────────────────────

/// A lightweight syntactic category tag. Identity is by string value so language
/// authors in different crates do not need to co-ordinate integer allocations.
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

// ── Compact arena indices ─────────────────────────────────────────────────────

/// Typed index into the `CstTree::nodes` arena.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SyntaxNodeId(pub u32);

/// Typed index into the `CstTree::tokens` arena.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SyntaxTokenId(pub u32);

// ── NodeOrToken ───────────────────────────────────────────────────────────────

/// Discriminated union of a node reference or a token reference.
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

// ── CstNodeChild ─────────────────────────────────────────────────────────────

/// One entry in a node's child list. Carries an optional field label.
#[derive(Debug, Clone)]
pub struct CstNodeChild {
    pub child: NodeOrToken,
    /// Optional field label from a `field("name", rule)` combinator.
    pub field_name: Option<&'static str>,
}

// ── CstNode ───────────────────────────────────────────────────────────────────

/// An interior node in the concrete syntax tree.
#[derive(Debug, Clone)]
pub struct CstNode {
    pub id: SyntaxNodeId,
    pub kind: SyntaxKind,
    pub span: SourceSpan,
    pub children: Vec<CstNodeChild>,
}

impl CstNode {
    /// Returns the first child with the given field name.
    pub fn field(&self, name: &str) -> Option<NodeOrToken> {
        self.children
            .iter()
            .find(|c| c.field_name == Some(name))
            .map(|c| c.child)
    }

    pub fn direct_tokens(&self) -> impl Iterator<Item = SyntaxTokenId> + '_ {
        self.children.iter().filter_map(|c| c.child.as_token())
    }

    pub fn direct_nodes(&self) -> impl Iterator<Item = SyntaxNodeId> + '_ {
        self.children.iter().filter_map(|c| c.child.as_node())
    }
}

// ── CstToken ──────────────────────────────────────────────────────────────────

/// A leaf token in the concrete syntax tree.
#[derive(Debug, Clone)]
pub struct CstToken {
    pub id: SyntaxTokenId,
    pub token_type: &'static str,
    pub token_sub_kind: Option<&'static str>,
    pub span: SourceSpan,
    /// `true` for whitespace, comments, and other trivia.
    pub is_trivia: bool,
}

impl CstToken {
    pub fn is_semantic(&self) -> bool {
        !self.is_trivia
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

    pub fn root(&self) -> &CstNode {
        self.node(self.root)
    }

    pub fn root_id(&self) -> SyntaxNodeId {
        self.root
    }

    // ── Direct arena access ───────────────────────────────────────────────────

    pub fn node(&self, id: SyntaxNodeId) -> &CstNode {
        &self.nodes[id.0 as usize]
    }

    pub fn token(&self, id: SyntaxTokenId) -> &CstToken {
        &self.tokens[id.0 as usize]
    }

    // ── Convenience helpers ───────────────────────────────────────────────────

    pub fn kind_of(&self, child: NodeOrToken) -> &str {
        match child {
            NodeOrToken::Node(id)  => self.node(id).kind.as_str(),
            NodeOrToken::Token(id) => self.token(id).token_type,
        }
    }

    pub fn span_of(&self, child: NodeOrToken) -> SourceSpan {
        match child {
            NodeOrToken::Node(id)  => self.node(id).span,
            NodeOrToken::Token(id) => self.token(id).span,
        }
    }

    pub fn field_node(&self, node_id: SyntaxNodeId, name: &str) -> Option<&CstNode> {
        let child = self.node(node_id).field(name)?;
        child.as_node().map(|id| self.node(id))
    }

    pub fn field_token(&self, node_id: SyntaxNodeId, name: &str) -> Option<&CstToken> {
        let child = self.node(node_id).field(name)?;
        child.as_token().map(|id| self.token(id))
    }

    pub fn nodes_of_kind(&self, kind: SyntaxKind) -> impl Iterator<Item = &CstNode> {
        self.nodes.iter().filter(move |n| n.kind == kind)
    }

    pub fn source_id(&self) -> SourceId {
        self.source_id
    }

    // ── Trivia helpers ────────────────────────────────────────────────────────

    pub fn semantic_tokens(&self) -> impl Iterator<Item = &CstToken> {
        self.tokens.iter().filter(|t| !t.is_trivia)
    }

    // ── Traversal helpers (used by visitors) ─────────────────────────────────

    pub fn children_of(&self, node_id: SyntaxNodeId) -> impl Iterator<Item = &CstNode> + '_ {
        self.node(node_id)
            .children
            .iter()
            .filter_map(|c| c.child.as_node().map(|id| self.node(id)))
    }

    pub fn tokens_of(&self, node_id: SyntaxNodeId) -> impl Iterator<Item = &CstToken> + '_ {
        self.node(node_id)
            .children
            .iter()
            .filter_map(|c| c.child.as_token().map(|id| self.token(id)))
            .filter(|t| !t.is_trivia)
    }

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

    pub fn walk(&self, visitor: &mut dyn crate::visitors::TreeVisitor) {
        self.walk_node(self.root, visitor);
    }

    pub fn walk_node(&self, node_id: SyntaxNodeId, visitor: &mut dyn crate::visitors::TreeVisitor) {
        let node = self.node(node_id);
        visitor.visit_node_enter(node, self);
        for child in &node.children.clone() {
            match child.child {
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
        let node = self.node(node_id);
        visitor.visit_node_exit(node, self);
    }
}

// ── Lowering trait ────────────────────────────────────────────────────────────

/// A post-parse transformation from `CstTree` to a typed AST.
pub trait AstLoweringStrategy {
    type Output;
    type Error;
    fn lower(&self, tree: &CstTree) -> Result<Self::Output, Self::Error>;
}
