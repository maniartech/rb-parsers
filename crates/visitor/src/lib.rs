//! Visitor API facade for `rb_parser` concrete syntax trees.
//!
//! Re-exports the [`TreeVisitor`] trait and [`DepthFirstWalker`] helper from
//! `rb_parser::visitors` so downstream consumers do not need to take a direct
//! dependency on the full parser crate.

pub use rb_parser::visitors::{DepthFirstWalker, TreeVisitor, WalkOrder};
