//! `rb_parser` — a PEG combinator parser framework producing concrete syntax trees.
//!
//! # Quick start
//!
//! ```rust,ignore
//! use rb_parser::prelude::*;
//! use rb_parser::profiles::ResolvedProfile;
//! use rb_common::diagnostics::DiagnosticsContext;
//!
//! #[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
//! enum MyRule { Root, Item }
//! impl RuleId for MyRule {}
//!
//! let compiled = grammar::<MyRule>()
//!     .rule(MyRule::Item, tok("WORD"))
//!     .rule(MyRule::Root, node(SyntaxKind::new("Root"), repeat1(ref_(MyRule::Item))))
//!     .start(MyRule::Root)
//!     .compile(&ResolvedProfile::simple("my_lang"))
//!     .unwrap();
//!
//! let mut ctx = DiagnosticsContext::new();
//! let tokens = vec![/* rb_tokenizer tokens */];
//! let tree = compiled.parse_tree(&tokens, &mut ctx);
//! ```

pub mod catalog;
pub mod cst;
pub mod engine;
pub mod events;
pub mod grammar;
pub mod profiles;
pub mod strategy;
pub mod visitors;

use rb_common::diagnostics::DiagnosticsContext;
use rb_common::spans::SourceId;
use rb_tokenizer::tokens::Token;

use crate::cst::CstTree;
use crate::engine::ParseContext;
use crate::events::ParseEvent;
use crate::grammar::{CompiledGrammar, RuleId, eval};
use crate::profiles::{RecoveryConfig, ResolvedProfile};
use crate::strategy::{CstBuildingStrategy, EventCollectingStrategy, ParseStrategy};

// ── CompiledParser ────────────────────────────────────────────────────────────

/// An immutable, compiled grammar ready to parse token streams.
///
/// `CompiledParser` is `Clone` and `Send + Sync`; each parse session creates
/// a fresh [`ParseContext`] on the stack.
pub struct CompiledParser {
    /// Type-erased execution function. We use a Box<dyn> to erase `R`.
    parse_fn: std::sync::Arc<dyn ParseFn>,
    pub(crate) profile: ResolvedProfile,
    pub(crate) recovery: RecoveryConfig,
}

impl Clone for CompiledParser {
    fn clone(&self) -> Self {
        CompiledParser {
            parse_fn: self.parse_fn.clone(),
            profile: self.profile.clone(),
            recovery: self.recovery.clone(),
        }
    }
}

// SAFETY: The parse function holds only `'static` grammar data.
unsafe impl Send for CompiledParser {}
unsafe impl Sync for CompiledParser {}

/// Type-erased parse function: executes the grammar over a token stream,
/// pushing events into a `FnMut(ParseEvent)` callback.
///
/// Two optional fast paths bypass the per-event virtual-dispatch overhead of
/// the callback interface:
/// - `run_building`    — monomorphised tree builder (used by `parse_tree`)
/// - `run_collecting`  — monomorphised event collector (used by `parse_events`)
trait ParseFn: Send + Sync {
    fn run(
        &self,
        tokens: &[Token],
        ctx: &mut DiagnosticsContext,
        profile: &ResolvedProfile,
        recovery: &RecoveryConfig,
        source_id: SourceId,
        emit: &mut dyn FnMut(ParseEvent),
    );

    /// Build a `CstTree` directly, without wrapping in a `dyn FnMut` callback.
    fn run_building(
        &self,
        tokens: &[Token],
        ctx: &mut DiagnosticsContext,
        profile: &ResolvedProfile,
        recovery: &RecoveryConfig,
        source_id: SourceId,
    ) -> CstTree;

    /// Collect all `ParseEvent`s directly, without wrapping in a `dyn FnMut` callback.
    fn run_collecting(
        &self,
        tokens: &[Token],
        ctx: &mut DiagnosticsContext,
        profile: &ResolvedProfile,
        recovery: &RecoveryConfig,
        source_id: SourceId,
        capacity_hint: usize,
    ) -> Vec<ParseEvent>;
}

struct GrammarParseFn<R: RuleId> {
    grammar: CompiledGrammar<R>,
}

impl<R: RuleId> ParseFn for GrammarParseFn<R> {
    fn run(
        &self,
        tokens: &[Token],
        ctx: &mut DiagnosticsContext,
        profile: &ResolvedProfile,
        recovery: &RecoveryConfig,
        source_id: SourceId,
        emit: &mut dyn FnMut(ParseEvent),
    ) {
        struct CallbackStrategy<'a>(&'a mut dyn FnMut(ParseEvent));
        impl ParseStrategy for CallbackStrategy<'_> {
            type Output = ();
            fn on_event(&mut self, event: ParseEvent) { (self.0)(event); }
            fn finish(self) {}
        }

        let mut parse_ctx = ParseContext::new(tokens, ctx, profile, source_id);
        let mut strategy = CallbackStrategy(emit);
        let mut recovery_steps = 0usize;

        let start_expr = &self.grammar.exprs[self.grammar.start_idx];
        let _ = eval(start_expr, &mut parse_ctx, &self.grammar, &mut strategy, None, &mut recovery_steps, recovery.max_recovery_steps);
    }

    fn run_building(
        &self,
        tokens: &[Token],
        ctx: &mut DiagnosticsContext,
        profile: &ResolvedProfile,
        recovery: &RecoveryConfig,
        source_id: SourceId,
    ) -> CstTree {
        let mut parse_ctx = ParseContext::new(tokens, ctx, profile, source_id);
        let mut strategy = CstBuildingStrategy::new(source_id);
        let mut recovery_steps = 0usize;
        let start_expr = &self.grammar.exprs[self.grammar.start_idx];
        let _ = eval(start_expr, &mut parse_ctx, &self.grammar, &mut strategy, None, &mut recovery_steps, recovery.max_recovery_steps);
        strategy.finish()
    }

    fn run_collecting(
        &self,
        tokens: &[Token],
        ctx: &mut DiagnosticsContext,
        profile: &ResolvedProfile,
        recovery: &RecoveryConfig,
        source_id: SourceId,
        capacity_hint: usize,
    ) -> Vec<ParseEvent> {
        let mut parse_ctx = ParseContext::new(tokens, ctx, profile, source_id);
        let mut strategy = EventCollectingStrategy::with_capacity(capacity_hint);
        let mut recovery_steps = 0usize;
        let start_expr = &self.grammar.exprs[self.grammar.start_idx];
        let _ = eval(start_expr, &mut parse_ctx, &self.grammar, &mut strategy, None, &mut recovery_steps, recovery.max_recovery_steps);
        strategy.finish()
    }
}

impl CompiledParser {
    /// Internal: construct a `CompiledParser` from a compiled grammar.
    pub(crate) fn from_grammar<R: RuleId>(
        grammar: CompiledGrammar<R>,
        profile: ResolvedProfile,
        recovery: RecoveryConfig,
    ) -> Self {
        CompiledParser {
            parse_fn: std::sync::Arc::new(GrammarParseFn { grammar }),
            profile,
            recovery,
        }
    }

    /// Parse a token stream and return a [`CstTree`].
    ///
    /// Uses `SourceId(0)`. For multi-file workspaces use [`Self::parse_tree_with_source`].
    pub fn parse_tree(&self, tokens: &[Token], ctx: &mut DiagnosticsContext) -> CstTree {
        self.parse_tree_with_source(tokens, ctx, SourceId(0))
    }

    /// Parse a token stream and return a [`CstTree`], tagging all spans with `source_id`.
    pub fn parse_tree_with_source(
        &self,
        tokens: &[Token],
        ctx: &mut DiagnosticsContext,
        source_id: SourceId,
    ) -> CstTree {
        self.parse_fn.run_building(tokens, ctx, &self.profile, &self.recovery, source_id)
    }

    /// Parse a token stream and return a flat `Vec<ParseEvent>`.
    ///
    /// Uses `SourceId(0)`. For multi-file workspaces use [`Self::parse_events_with_source`].
    pub fn parse_events(&self, tokens: &[Token], ctx: &mut DiagnosticsContext) -> Vec<ParseEvent> {
        self.parse_events_with_source(tokens, ctx, SourceId(0))
    }

    /// Parse a token stream and return a flat `Vec<ParseEvent>`, tagging all spans with `source_id`.
    pub fn parse_events_with_source(
        &self,
        tokens: &[Token],
        ctx: &mut DiagnosticsContext,
        source_id: SourceId,
    ) -> Vec<ParseEvent> {
        // Heuristic: each token generates ~4 events on average (NodeStart,
        // Token, NodeEnd, plus occasional FieldStart/FieldEnd).  Pre-allocating
        // avoids repeated Vec growth and the associated memcpy overhead.
        let capacity = tokens.len().saturating_mul(4);
        self.parse_fn.run_collecting(tokens, ctx, &self.profile, &self.recovery, source_id, capacity)
    }

    /// Parse using a custom [`ParseStrategy`].
    pub fn parse_with_strategy<S: ParseStrategy>(
        &self,
        tokens: &[Token],
        ctx: &mut DiagnosticsContext,
        strategy: S,
    ) -> S::Output {
        self.parse_with_strategy_and_source(tokens, ctx, strategy, SourceId(0))
    }

    /// Parse using a custom [`ParseStrategy`], tagging all spans with `source_id`.
    pub fn parse_with_strategy_and_source<S: ParseStrategy>(
        &self,
        tokens: &[Token],
        ctx: &mut DiagnosticsContext,
        mut strategy: S,
        source_id: SourceId,
    ) -> S::Output {
        self.parse_fn.run(tokens, ctx, &self.profile, &self.recovery, source_id,
            &mut |event| strategy.on_event(event));
        strategy.finish()
    }

    /// Build a stateful [`IncrementalParser`] backed by this compiled grammar.
    pub fn incremental(&self) -> IncrementalParser<'_> {
        IncrementalParser { compiled: self, cached_tree: None }
    }
}

// ── IncrementalParser ─────────────────────────────────────────────────────────

pub struct IncrementalParser<'compiled> {
    compiled: &'compiled CompiledParser,
    cached_tree: Option<CstTree>,
}

impl<'compiled> IncrementalParser<'compiled> {
    pub fn initial_parse(
        &mut self,
        tokens: &[Token],
        ctx: &mut DiagnosticsContext,
    ) -> &CstTree {
        let tree = self.compiled.parse_tree(tokens, ctx);
        self.cached_tree = Some(tree);
        self.cached_tree.as_ref().unwrap()
    }

    /// Re-parse the entire token stream after edits.
    ///
    /// **Phase 1 note**: This is a full re-parse. The `edits` parameter is
    /// accepted for API stability but not yet used to narrow the reparse region.
    /// True incremental reuse (reusing subtrees unaffected by edits) is deferred
    /// to Phase 2. Callers may pass `&[]` if they do not have edit metadata.
    pub fn reparse(
        &mut self,
        tokens: &[Token],
        _edits: &[TextEdit],
        ctx: &mut DiagnosticsContext,
    ) -> &CstTree {
        self.initial_parse(tokens, ctx)
    }

    pub fn invalidate(&mut self) {
        self.cached_tree = None;
    }

    pub fn cached_tree(&self) -> Option<&CstTree> {
        self.cached_tree.as_ref()
    }
}

// ── TextEdit ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEdit {
    pub range: std::ops::Range<usize>,
    pub replacement: String,
}

impl TextEdit {
    pub fn insert(at: usize, text: impl Into<String>) -> Self {
        TextEdit { range: at..at, replacement: text.into() }
    }
    pub fn delete(range: std::ops::Range<usize>) -> Self {
        TextEdit { range, replacement: String::new() }
    }
    pub fn replace(range: std::ops::Range<usize>, text: impl Into<String>) -> Self {
        TextEdit { range, replacement: text.into() }
    }
}

// ── Prelude ───────────────────────────────────────────────────────────────────

pub mod prelude {
    pub use crate::grammar::{
        GrammarRule, RuleId, RecoveryLandmarks, GrammarError,
        tok, tok_sub, ref_, seq2, alt2, repeat0, repeat1, opt, cut,
        node, field, between, list, list1, pratt, grammar,
        PrattBuilder, GrammarBuilder, Grammar,
    };
    pub use crate::cst::SyntaxKind;
    pub use crate::profiles::{ResolvedProfile, ProfileMode, profile_guard, profile};
    pub use crate::{seq, one_of, any_of};
}
