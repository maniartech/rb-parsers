use crate::cst::{CstNode, CstToken, CstTree};

// ── WalkOrder ─────────────────────────────────────────────────────────────────

/// Controls traversal order. `BreadthFirst` is reserved for Phase 2.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WalkOrder {
    DepthFirst,
}

// ── TreeVisitor ───────────────────────────────────────────────────────────────

/// Traversal callback interface over a [`CstTree`].
///
/// All methods have empty default implementations so implementors only need to
/// override the hooks they care about.
pub trait TreeVisitor {
    fn visit_node_enter(&mut self, _node: &CstNode, _tree: &CstTree) {}
    fn visit_node_exit(&mut self, _node: &CstNode, _tree: &CstTree) {}
    fn visit_token(&mut self, _token: &CstToken, _tree: &CstTree) {}
    fn visit_trivia_token(&mut self, _token: &CstToken, _tree: &CstTree) {}
}

// ── DepthFirstWalker ──────────────────────────────────────────────────────────

/// Drives a [`TreeVisitor`] over a [`CstTree`] in depth-first order.
pub struct DepthFirstWalker<'a, V: TreeVisitor> {
    pub visitor: &'a mut V,
}

impl<'a, V: TreeVisitor> DepthFirstWalker<'a, V> {
    pub fn new(visitor: &'a mut V) -> Self {
        DepthFirstWalker { visitor }
    }

    pub fn walk(&mut self, tree: &CstTree) {
        tree.walk(self.visitor);
    }

    pub fn walk_from(
        &mut self,
        tree: &CstTree,
        node_id: crate::cst::SyntaxNodeId,
    ) {
        tree.walk_node(node_id, self.visitor);
    }
}
