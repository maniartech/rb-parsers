#![warn(missing_docs)]
//! Visitor API facade for `rb_parser` concrete syntax trees.
//!
//! Re-exports visitor types from `rb_parser::visitors` so downstream consumers
//! do not need a direct dependency on the full parser crate.

pub use rb_parser::visitors::{
    DepthFirstWalker, FilteringVisitor, KindVisitor, SpanCollector,
    StreamVisitor, StreamVisitorStrategy, SyntaxCursor, TreeVisitor,
    TriviafreeVisitor, WalkOrder,
};
