use crate::events::ParseEvent;

/// Push-model consumer of `ParseEvent`s.
pub trait ParseStrategy: Sized {
    /// The value produced when the parse is complete.
    type Output;
    /// Receives the next event from the parse engine.
    fn on_event(&mut self, event: ParseEvent);
    /// Called once parsing is finished; consumes `self` and returns the output.
    fn finish(self) -> Self::Output;
}

// ── CstBuildingStrategy ───────────────────────────────────────────────────────

use rb_common::spans::{SourceId, SourcePosition, SourceSpan};
use crate::cst::{
    CstNode, CstNodeChild, CstToken, CstTree, NodeOrToken, SyntaxKind,
    SyntaxNodeId, SyntaxTokenId,
};

/// A [`ParseStrategy`] that builds a [`CstTree`] from the event stream.
pub struct CstBuildingStrategy {
    source_id: SourceId,
    nodes: Vec<CstNode>,
    tokens: Vec<CstToken>,
    /// Stack of open node builders: (kind, span_start, arena_start).
    ///
    /// `arena_start` is the index into `children_arena` where this node's
    /// children begin.  On `NodeEnd` we drain `arena_start..` from the arena
    /// into the finished `CstNode`, then push the new node back as a child of
    /// the parent.  This eliminates one `Vec` allocation per node, which is
    /// the dominant cost for wide / deep parse trees.
    stack: Vec<(SyntaxKind, SourcePosition, usize)>,
    /// Flat arena for in-progress node children.  All open nodes share this
    /// single backing allocation; each node owns a contiguous slice denoted by
    /// `arena_start..children_arena.len()` at the time it is closed.
    children_arena: Vec<CstNodeChild>,
    /// Current field name context
    field_stack: Vec<&'static str>,
}

impl CstBuildingStrategy {
    /// Creates a new strategy for the given `source_id`.
    pub fn new(source_id: SourceId) -> Self {
        CstBuildingStrategy {
            source_id,
            nodes: Vec::new(),
            tokens: Vec::new(),
            stack: Vec::new(),
            children_arena: Vec::new(),
            field_stack: Vec::new(),
        }
    }
}

impl ParseStrategy for CstBuildingStrategy {
    type Output = CstTree;

    fn on_event(&mut self, event: ParseEvent) {
        match event {
            ParseEvent::NodeStart { kind, span_start } => {
                // Record where this node's children will start in the arena.
                self.stack.push((kind, span_start, self.children_arena.len()));
            }
            ParseEvent::NodeEnd { kind: _, span } => {
                if let Some((kind, _start, arena_start)) = self.stack.pop() {
                    // Collect all children accumulated in the arena since NodeStart.
                    let children: Vec<CstNodeChild> =
                        self.children_arena.drain(arena_start..).collect();
                    let id = SyntaxNodeId(self.nodes.len() as u32);
                    let node = CstNode { id, kind, span, children, is_error_recovery: false };
                    self.nodes.push(node);
                    // Register completed node as a child of the enclosing node.
                    let not = NodeOrToken::Node(id);
                    let field_name = self.field_stack.last().copied();
                    if self.stack.last().is_some() {
                        self.children_arena.push(CstNodeChild { child: not, field_name });
                    }
                }
            }
            ParseEvent::Token { token_type, token_sub_kind, span, is_trivia, field_name } => {
                let id = SyntaxTokenId(self.tokens.len() as u32);
                self.tokens.push(CstToken { id, token_type, token_sub_kind, span, is_trivia });
                let effective_field = field_name.or_else(|| self.field_stack.last().copied());
                if self.stack.last().is_some() {
                    self.children_arena.push(CstNodeChild {
                        child: NodeOrToken::Token(id),
                        field_name: effective_field,
                    });
                }
            }
            ParseEvent::FieldStart { name } => {
                self.field_stack.push(name);
            }
            ParseEvent::FieldEnd { .. } => {
                self.field_stack.pop();
            }
            ParseEvent::Error { .. } | ParseEvent::Recovery { .. } => {
                // Diagnostics handled by ParseContext; we do not embed them in the tree here.
            }
        }
    }

    fn finish(mut self) -> CstTree {
        // If the stack still has entries (e.g. top-level node), close them.
        while self.stack.len() > 1 {
            if let Some((kind, start, arena_start)) = self.stack.pop() {
                let children: Vec<CstNodeChild> =
                    self.children_arena.drain(arena_start..).collect();
                let id = SyntaxNodeId(self.nodes.len() as u32);
                let span = SourceSpan::new(self.source_id, start, start);
                let node = CstNode { id, kind, span, children, is_error_recovery: true };
                self.nodes.push(node);
                let not = NodeOrToken::Node(id);
                if self.stack.last().is_some() {
                    self.children_arena
                        .push(CstNodeChild { child: not, field_name: None });
                }
            }
        }
        // Last entry is the root
        let root_id = if let Some((kind, start, arena_start)) = self.stack.pop() {
            let children: Vec<CstNodeChild> =
                self.children_arena.drain(arena_start..).collect();
            let id = SyntaxNodeId(self.nodes.len() as u32);
            let end = self.tokens.last().map(|t| t.span.end).unwrap_or(start);
            let span = SourceSpan::new(self.source_id, start, end);
            self.nodes.push(CstNode { id, kind, span, children, is_error_recovery: false });
            id
        } else if !self.nodes.is_empty() {
            SyntaxNodeId((self.nodes.len() - 1) as u32)
        } else {
            // Empty parse — synthesize a dummy root
            let id = SyntaxNodeId(0);
            self.nodes.push(CstNode {
                id,
                kind: SyntaxKind::new("Root"),
                span: SourceSpan::UNKNOWN,
                children: Vec::new(),
                is_error_recovery: false,
            });
            id
        };

        CstTree::new(self.nodes, self.tokens, root_id, self.source_id)
    }
}

// ── EventCollectingStrategy ───────────────────────────────────────────────────

/// A [`ParseStrategy`] that simply collects all events into a `Vec<ParseEvent>`.
pub struct EventCollectingStrategy {
    events: Vec<ParseEvent>,
}

impl EventCollectingStrategy {
    /// Creates a new strategy with an empty event buffer.
    pub fn new() -> Self { EventCollectingStrategy { events: Vec::new() } }

    /// Pre-allocate for `capacity` events to avoid reallocation during parsing.
    /// A heuristic of `tokens * 4` covers most real grammars (NodeStart,
    /// Token, and NodeEnd events per token, plus field markers).
    pub fn with_capacity(capacity: usize) -> Self {
        EventCollectingStrategy { events: Vec::with_capacity(capacity) }
    }
}

impl Default for EventCollectingStrategy {
    fn default() -> Self { Self::new() }
}

impl ParseStrategy for EventCollectingStrategy {
    type Output = Vec<ParseEvent>;
    fn on_event(&mut self, event: ParseEvent) { self.events.push(event); }
    fn finish(self) -> Vec<ParseEvent> { self.events }
}
