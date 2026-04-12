use crate::cst::{CstNode, CstToken, CstTree, NodeOrToken, SyntaxKind, SyntaxNodeId, SyntaxTokenId};

// ── WalkOrder ─────────────────────────────────────────────────────────────────

/// Controls traversal order. `BreadthFirst` is reserved for Phase 2.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WalkOrder {
    /// Visits each node before its children.
    DepthFirst,
}

// ── TreeVisitor ───────────────────────────────────────────────────────────────

/// Traversal callback interface over a [`CstTree`].
///
/// All methods have empty default implementations so implementors only need to
/// override the hooks they care about.
pub trait TreeVisitor {
    /// Called when the walker descends into a node.
    fn visit_node_enter(&mut self, _node: &CstNode, _tree: &CstTree) {}
    /// Called when the walker ascends out of a node.
    fn visit_node_exit(&mut self, _node: &CstNode, _tree: &CstTree) {}
    /// Called for each non-trivia leaf token.
    fn visit_token(&mut self, _token: &CstToken, _tree: &CstTree) {}
    /// Called for each trivia (whitespace / comment) leaf token.
    fn visit_trivia_token(&mut self, _token: &CstToken, _tree: &CstTree) {}
}

// ── DepthFirstWalker ──────────────────────────────────────────────────────────

/// Drives a [`TreeVisitor`] over a [`CstTree`] in depth-first order.
pub struct DepthFirstWalker<'a, V: TreeVisitor> {
    /// The wrapped visitor.
    pub visitor: &'a mut V,
}

impl<'a, V: TreeVisitor> DepthFirstWalker<'a, V> {
    /// Creates a new walker that will drive `visitor`.
    pub fn new(visitor: &'a mut V) -> Self {
        DepthFirstWalker { visitor }
    }

    /// Walks the entire tree from its root.
    pub fn walk(&mut self, tree: &CstTree) {
        tree.walk(self.visitor);
    }

    /// Walks the subtree rooted at `node_id`.
    pub fn walk_from(
        &mut self,
        tree: &CstTree,
        node_id: crate::cst::SyntaxNodeId,
    ) {
        tree.walk_node(node_id, self.visitor);
    }
}

// ── KindVisitor (E3) ──────────────────────────────────────────────────────────

/// A [`TreeVisitor`] that dispatches enter/exit callbacks by [`SyntaxKind`].
///
/// Handlers are registered with [`on_enter`](Self::on_enter) and
/// [`on_exit`](Self::on_exit) and are only invoked for the matching kind.
/// Unhandled kinds are silently skipped.
///
/// # Example
/// ```rust,ignore
/// let mut visitor = KindVisitor::new()
///     .on_enter(SyntaxKind::new("FunctionDef"), |node, tree| {
///         println!("fn at {:?}", node.span);
///     });
/// DepthFirstWalker::new(&mut visitor).walk(&tree);
/// ```
pub struct KindVisitor<'h> {
    enter_handlers: Vec<(SyntaxKind, Box<dyn Fn(&CstNode, &CstTree) + 'h>)>,
    exit_handlers:  Vec<(SyntaxKind, Box<dyn Fn(&CstNode, &CstTree) + 'h>)>,
}

impl<'h> KindVisitor<'h> {
    /// Create an empty visitor.
    pub fn new() -> Self {
        KindVisitor { enter_handlers: Vec::new(), exit_handlers: Vec::new() }
    }

    /// Register a callback invoked when entering a node of `kind`.
    pub fn on_enter(mut self, kind: SyntaxKind, f: impl Fn(&CstNode, &CstTree) + 'h) -> Self {
        self.enter_handlers.push((kind, Box::new(f)));
        self
    }

    /// Register a callback invoked when exiting a node of `kind`.
    pub fn on_exit(mut self, kind: SyntaxKind, f: impl Fn(&CstNode, &CstTree) + 'h) -> Self {
        self.exit_handlers.push((kind, Box::new(f)));
        self
    }
}

impl<'h> Default for KindVisitor<'h> {
    fn default() -> Self { Self::new() }
}

impl<'h> TreeVisitor for KindVisitor<'h> {
    fn visit_node_enter(&mut self, node: &CstNode, tree: &CstTree) {
        for (kind, handler) in &self.enter_handlers {
            if *kind == node.kind { handler(node, tree); }
        }
    }

    fn visit_node_exit(&mut self, node: &CstNode, tree: &CstTree) {
        for (kind, handler) in &self.exit_handlers {
            if *kind == node.kind { handler(node, tree); }
        }
    }
}

// ── SyntaxCursor (E4) ─────────────────────────────────────────────────────────

/// A lightweight, zero-allocation cursor over a [`CstTree`].
///
/// A cursor holds a reference to the tree and a `SyntaxNodeId`. Navigation
/// methods return new cursors pointing at neighbouring nodes without modifying
/// the tree.
///
/// **Note**: Operations that require parent navigation (`parent()`,
/// `prev_sibling()`, `ancestor_of_kind()`) are O(N) in the number of nodes
/// because the current `CstNode` layout does not store a parent pointer. A
/// future `CstTree::with_parent_table()` API can make them O(1).
///
/// # Example
/// ```rust,ignore
/// let cursor = SyntaxCursor::new(&tree, tree.root_id());
/// if let Some(fn_cursor) = cursor.child_of_kind(SyntaxKind::new("FunctionDef")) {
///     println!("first function: {:?}", fn_cursor.span());
/// }
/// ```
pub struct SyntaxCursor<'t> {
    tree:    &'t CstTree,
    node_id: SyntaxNodeId,
}

impl<'t> SyntaxCursor<'t> {
    /// Create a cursor pointing at `node_id`.
    pub fn new(tree: &'t CstTree, node_id: SyntaxNodeId) -> Self {
        SyntaxCursor { tree, node_id }
    }

    /// Create a cursor at the root of the tree.
    pub fn root(tree: &'t CstTree) -> Self {
        SyntaxCursor { tree, node_id: tree.root_id() }
    }

    // ── Current node ─────────────────────────────────────────────────────────

    /// Returns the `SyntaxNodeId` of the current node.
    pub fn node_id(&self) -> SyntaxNodeId { self.node_id }
    /// Returns a reference to the current `CstNode`.
    pub fn node(&self) -> &'t CstNode { self.tree.node(self.node_id) }
    /// Returns the `SyntaxKind` of the current node.
    pub fn kind(&self) -> SyntaxKind { self.tree.node(self.node_id).kind }
    /// Returns the source span covered by the current node.
    pub fn span(&self) -> rb_common::spans::SourceSpan { self.tree.node(self.node_id).span }
    /// Returns `true` if the current node was inserted by the error-recovery system.
    pub fn is_error_recovery(&self) -> bool { self.tree.node(self.node_id).is_error_recovery }

    // ── Children ─────────────────────────────────────────────────────────────

    /// Iterate over all semantic-node children of the current node.
    pub fn children(&self) -> impl Iterator<Item = SyntaxCursor<'t>> + '_ {
        let tree = self.tree;
        self.tree.node(self.node_id)
            .children
            .iter()
            .filter_map(move |c| c.child.as_node().map(|id| SyntaxCursor { tree, node_id: id }))
    }

    /// Return the first child node, if any.
    pub fn first_child(&self) -> Option<SyntaxCursor<'t>> {
        let tree = self.tree;
        self.tree.node(self.node_id)
            .children
            .iter()
            .find_map(|c| c.child.as_node().map(|id| SyntaxCursor { tree, node_id: id }))
    }

    /// Return the last child node, if any.
    pub fn last_child(&self) -> Option<SyntaxCursor<'t>> {
        let tree = self.tree;
        self.tree.node(self.node_id)
            .children
            .iter()
            .rev()
            .find_map(|c| c.child.as_node().map(|id| SyntaxCursor { tree, node_id: id }))
    }

    /// Return the first child node of the given `kind`.
    pub fn child_of_kind(&self, kind: SyntaxKind) -> Option<SyntaxCursor<'t>> {
        let tree = self.tree;
        self.tree.node(self.node_id)
            .children
            .iter()
            .find_map(|c| {
                c.child.as_node().and_then(|id| {
                    if tree.node(id).kind == kind {
                        Some(SyntaxCursor { tree, node_id: id })
                    } else {
                        None
                    }
                })
            })
    }

    /// Return the child node with the given field label.
    pub fn field(&self, name: &str) -> Option<SyntaxCursor<'t>> {
        let tree = self.tree;
        self.tree.node(self.node_id)
            .children
            .iter()
            .find_map(|c| {
                if c.field_name == Some(name) {
                    c.child.as_node().map(|id| SyntaxCursor { tree, node_id: id })
                } else {
                    None
                }
            })
    }

    // ── Sibling navigation (O(N) — requires scanning parent) ────────────────

    /// Return the next sibling node, or `None` if there is none.
    /// O(N) because the tree does not store parent pointers.
    pub fn next_sibling(&self) -> Option<SyntaxCursor<'t>> {
        self.parent()?.sibling_after(self.node_id)
    }

    /// Return the previous sibling node, or `None` if there is none.
    /// O(N) because the tree does not store parent pointers.
    pub fn prev_sibling(&self) -> Option<SyntaxCursor<'t>> {
        self.parent()?.sibling_before(self.node_id)
    }

    // ── Parent navigation (O(N)) ─────────────────────────────────────────────

    /// Return the parent node, or `None` for the tree root.
    /// O(N) because the tree does not store parent pointers.
    pub fn parent(&self) -> Option<SyntaxCursor<'t>> {
        let target = self.node_id;
        let tree = self.tree;
        // Linear scan: find any node that has `target` as a child.
        tree.node_ids()
            .find(|&parent_id| {
                tree.node(parent_id)
                    .children
                    .iter()
                    .any(|c| c.child == NodeOrToken::Node(target))
            })
            .map(|node_id| SyntaxCursor { tree, node_id })
    }

    /// Find the nearest ancestor (inclusive) of the given `kind`.
    /// O(depth × N) due to O(N) `parent()`.
    pub fn ancestor_of_kind(&self, kind: SyntaxKind) -> Option<SyntaxCursor<'t>> {
        if self.kind() == kind { return Some(SyntaxCursor { tree: self.tree, node_id: self.node_id }); }
        self.parent()?.ancestor_of_kind(kind)
    }

    // ── Token access ─────────────────────────────────────────────────────────

    /// Return the first semantic (non-trivia) token child of this node.
    pub fn first_token(&self) -> Option<(SyntaxTokenId, &'t CstToken)> {
        self.tree.node(self.node_id)
            .children
            .iter()
            .find_map(|c| {
                c.child.as_token().and_then(|id| {
                    let tok = self.tree.token(id);
                    if !tok.is_trivia { Some((id, tok)) } else { None }
                })
            })
    }

    /// Return the source text of the first semantic token child, if present.
    pub fn text<'s>(&self, source: &'s str) -> Option<&'s str> {
        self.first_token().map(|(_, tok)| tok.text(source))
    }

    // ── Private helpers ───────────────────────────────────────────────────────

    fn sibling_after(&self, target: SyntaxNodeId) -> Option<SyntaxCursor<'t>> {
        let tree = self.tree;
        let children = &self.tree.node(self.node_id).children;
        let mut found = false;
        for c in children {
            if let Some(id) = c.child.as_node() {
                if found { return Some(SyntaxCursor { tree, node_id: id }); }
                if id == target { found = true; }
            }
        }
        None
    }

    fn sibling_before(&self, target: SyntaxNodeId) -> Option<SyntaxCursor<'t>> {
        let tree = self.tree;
        let children = &self.tree.node(self.node_id).children;
        let mut prev: Option<SyntaxNodeId> = None;
        for c in children {
            if let Some(id) = c.child.as_node() {
                if id == target { return prev.map(|node_id| SyntaxCursor { tree, node_id }); }
                prev = Some(id);
            }
        }
        None
    }
}

impl<'t> std::fmt::Debug for SyntaxCursor<'t> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SyntaxCursor({:?}, {:?})", self.node_id, self.kind())
    }
}

// ── TriviafreeVisitor (C19) ───────────────────────────────────────────────────

/// A [`TreeVisitor`] adaptor that forwards node callbacks to `inner` but
/// suppresses calls for trivia tokens (whitespace, comments, etc.).
///
/// The `visit_trivia_token` callback is still forwarded if you want to inspect
/// trivia; only the semantic-token / node separation is preserved.
///
/// # Example
/// ```rust,ignore
/// let mut counter = TokenCounter::default();
/// let mut visitor = TriviafreeVisitor::new(&mut counter);
/// DepthFirstWalker::new(&mut visitor).walk(&tree);
/// ```
pub struct TriviafreeVisitor<'a, V: TreeVisitor> {
    inner: &'a mut V,
}

impl<'a, V: TreeVisitor> TriviafreeVisitor<'a, V> {
    /// Wraps `inner`, filtering out trivia tokens from `visit_token` calls.
    pub fn new(inner: &'a mut V) -> Self {
        TriviafreeVisitor { inner }
    }
}

impl<'a, V: TreeVisitor> TreeVisitor for TriviafreeVisitor<'a, V> {
    fn visit_node_enter(&mut self, node: &CstNode, tree: &CstTree) {
        self.inner.visit_node_enter(node, tree);
    }

    fn visit_node_exit(&mut self, node: &CstNode, tree: &CstTree) {
        self.inner.visit_node_exit(node, tree);
    }

    fn visit_token(&mut self, token: &CstToken, tree: &CstTree) {
        // Only forward non-trivia tokens.
        if !token.is_trivia {
            self.inner.visit_token(token, tree);
        }
    }

    fn visit_trivia_token(&mut self, token: &CstToken, tree: &CstTree) {
        self.inner.visit_trivia_token(token, tree);
    }
}

// ── FilteringVisitor (C19) ────────────────────────────────────────────────────

/// A [`TreeVisitor`] adaptor that only forwards `visit_node_enter` /
/// `visit_node_exit` calls that satisfy a user-supplied predicate.
///
/// Tokens always pass through to the inner visitor unchanged.
///
/// # Example
/// ```rust,ignore
/// let mut handler = MyHandler::default();
/// let mut visitor = FilteringVisitor::new(&mut handler, |node, _| {
///     node.kind.as_str() != "Whitespace"
/// });
/// DepthFirstWalker::new(&mut visitor).walk(&tree);
/// ```
pub struct FilteringVisitor<'a, V, F>
where
    V: TreeVisitor,
    F: Fn(&CstNode, &CstTree) -> bool,
{
    inner:     &'a mut V,
    predicate: F,
}

impl<'a, V, F> FilteringVisitor<'a, V, F>
where
    V: TreeVisitor,
    F: Fn(&CstNode, &CstTree) -> bool,
{
    /// Wraps `inner`, only forwarding node events that satisfy `predicate`.
    pub fn new(inner: &'a mut V, predicate: F) -> Self {
        FilteringVisitor { inner, predicate }
    }
}

impl<'a, V, F> TreeVisitor for FilteringVisitor<'a, V, F>
where
    V: TreeVisitor,
    F: Fn(&CstNode, &CstTree) -> bool,
{
    fn visit_node_enter(&mut self, node: &CstNode, tree: &CstTree) {
        if (self.predicate)(node, tree) {
            self.inner.visit_node_enter(node, tree);
        }
    }

    fn visit_node_exit(&mut self, node: &CstNode, tree: &CstTree) {
        if (self.predicate)(node, tree) {
            self.inner.visit_node_exit(node, tree);
        }
    }

    fn visit_token(&mut self, token: &CstToken, tree: &CstTree) {
        self.inner.visit_token(token, tree);
    }

    fn visit_trivia_token(&mut self, token: &CstToken, tree: &CstTree) {
        self.inner.visit_trivia_token(token, tree);
    }
}

// ── SpanCollector (C19) ───────────────────────────────────────────────────────

use rb_common::spans::SourceSpan;

/// A [`TreeVisitor`] that collects node spans filtered by `SyntaxKind`.
///
/// After walking, call [`spans()`](SpanCollector::spans) to retrieve all matched spans.
///
/// # Example
/// ```rust,ignore
/// let mut collector = SpanCollector::for_kind(SyntaxKind::new("FunctionDef"));
/// DepthFirstWalker::new(&mut collector).walk(&tree);
/// for span in collector.spans() {
///     println!("function at {:?}", span);
/// }
/// ```
pub struct SpanCollector {
    kind:    Option<SyntaxKind>,
    results: Vec<(SyntaxKind, SourceSpan)>,
}

impl SpanCollector {
    /// Collect spans for all node kinds.
    pub fn all() -> Self {
        SpanCollector { kind: None, results: Vec::new() }
    }

    /// Collect spans only for nodes matching `kind`.
    pub fn for_kind(kind: SyntaxKind) -> Self {
        SpanCollector { kind: Some(kind), results: Vec::new() }
    }

    /// Return all collected `(SyntaxKind, SourceSpan)` pairs.
    pub fn spans(&self) -> &[(SyntaxKind, SourceSpan)] {
        &self.results
    }

    /// Drain and return collected spans.
    pub fn take_spans(&mut self) -> Vec<(SyntaxKind, SourceSpan)> {
        std::mem::take(&mut self.results)
    }
}

impl TreeVisitor for SpanCollector {
    fn visit_node_enter(&mut self, node: &CstNode, _tree: &CstTree) {
        let matches = match self.kind {
            Some(k) => k == node.kind,
            None    => true,
        };
        if matches {
            self.results.push((node.kind, node.span));
        }
    }
}

// ── StreamVisitor / StreamVisitorStrategy (E6) ───────────────────────────────

use rb_common::diagnostics::Diagnostic;
use rb_common::spans::SourcePosition;
use crate::events::ParseEvent;
use crate::strategy::ParseStrategy;

/// Lightweight event-driven visitor that receives parse events directly from
/// the engine. Because no CST is materialised, implementing `StreamVisitor`
/// is the fastest way to run a single-pass analysis or extraction.
///
/// All methods have default no-op implementations; override only what you need.
///
/// # Example
/// ```rust,ignore
/// use rb_parser::visitors::{StreamVisitor, StreamVisitorStrategy};
///
/// struct Counter { nodes: usize }
///
/// impl StreamVisitor for Counter {
///     fn on_node_enter(&mut self, _kind: SyntaxKind, _start: SourcePosition) {
///         self.nodes += 1;
///     }
/// }
///
/// let strategy = StreamVisitorStrategy::new(Counter { nodes: 0 });
/// let counter  = parser.parse_with_strategy(tokens, strategy);
/// println!("node count: {}", counter.nodes);
/// ```
pub trait StreamVisitor {
    /// Called when a syntax node opens.
    fn on_node_enter(&mut self, _kind: SyntaxKind, _span_start: SourcePosition) {}
    /// Called when a syntax node closes.
    fn on_node_exit(&mut self, _kind: SyntaxKind, _span: SourceSpan) {}
    /// Called when the engine consumes a leaf token.
    fn on_token(
        &mut self,
        _token_type: &'static str,
        _span: SourceSpan,
        _is_trivia: bool,
    ) {
    }
    /// Called when a diagnostic is emitted during parsing.
    fn on_error(&mut self, _diagnostic: &Diagnostic) {}
}

/// Adapts a [`StreamVisitor`] so it can be passed as a [`ParseStrategy`] to
/// [`CompiledParser::parse_with_strategy`].
///
/// When the parse finishes, `finish()` returns the inner visitor unchanged so
/// callers can read any state accumulated during the walk.
pub struct StreamVisitorStrategy<V: StreamVisitor> {
    /// The wrapped visitor.
    pub visitor: V,
}

impl<V: StreamVisitor> StreamVisitorStrategy<V> {
    /// Wraps `visitor` in a strategy adapter.
    pub fn new(visitor: V) -> Self {
        StreamVisitorStrategy { visitor }
    }
}

impl<V: StreamVisitor> ParseStrategy for StreamVisitorStrategy<V> {
    type Output = V;

    fn on_event(&mut self, event: ParseEvent) {
        match event {
            ParseEvent::NodeStart { kind, span_start } => {
                self.visitor.on_node_enter(kind, span_start);
            }
            ParseEvent::NodeEnd { kind, span } => {
                self.visitor.on_node_exit(kind, span);
            }
            ParseEvent::Token { token_type, span, is_trivia, .. } => {
                self.visitor.on_token(token_type, span, is_trivia);
            }
            ParseEvent::Error { diagnostic } => {
                self.visitor.on_error(&diagnostic);
            }
            // FieldStart / FieldEnd / Recovery have no StreamVisitor hook.
            _ => {}
        }
    }

    fn finish(self) -> V {
        self.visitor
    }
}

