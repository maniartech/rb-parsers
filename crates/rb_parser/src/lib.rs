#![warn(missing_docs)]
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

/// Error catalog — machine-readable codes for parser diagnostics.
pub mod catalog;
/// Concrete syntax tree — `CstTree`, `CstNode`, `CstToken`, `SyntaxKind`, and friends.
pub mod cst;
/// Parse engine — the recursive-descent driver that evaluates grammars.
pub mod engine;
/// Parse events — the low-level event stream produced during a parse.
pub mod events;
/// Grammar DSL — combinators and builders for defining grammars.
pub mod grammar;
/// Parsing profiles — language mode and recovery configuration.
pub mod profiles;
/// Parse strategies — output-determining callbacks (`CstBuildingStrategy`, etc.).
pub mod strategy;
/// Visitor API — tree walkers and event-based visitor types.
pub mod visitors;

use rb_common::diagnostics::DiagnosticsContext;
use rb_common::spans::SourceId;
use rb_tokenizer::token_source::{BufferedTokenSource, SliceTokenSource};
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

// `CompiledParser` is `Send + Sync` automatically because:
// - `parse_fn: Arc<dyn ParseFn>` where `ParseFn: Send + Sync`
// - `profile` and `recovery` are `Send + Sync`
// The manual unsafe impls below are removed — the compiler now verifies this.

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
        tokens: &[Token<'_>],
        ctx: &mut DiagnosticsContext,
        profile: &ResolvedProfile,
        recovery: &RecoveryConfig,
        source_id: SourceId,
        emit: &mut dyn FnMut(ParseEvent),
    );

    /// Build a `CstTree` directly, without wrapping in a `dyn FnMut` callback.
    fn run_building(
        &self,
        tokens: &[Token<'_>],
        ctx: &mut DiagnosticsContext,
        profile: &ResolvedProfile,
        recovery: &RecoveryConfig,
        source_id: SourceId,
    ) -> CstTree;

    /// Collect all `ParseEvent`s directly, without wrapping in a `dyn FnMut` callback.
    fn run_collecting(
        &self,
        tokens: &[Token<'_>],
        ctx: &mut DiagnosticsContext,
        profile: &ResolvedProfile,
        recovery: &RecoveryConfig,
        source_id: SourceId,
        capacity_hint: usize,
    ) -> Vec<ParseEvent>;

    /// Returns all rule names registered in the grammar (in definition order).
    fn rule_names(&self) -> Vec<String>;

    /// Returns the FIRST set of the start rule — the set of token types that
    /// can legitimately begin a parse.
    fn first_set_start(&self) -> std::collections::HashSet<&'static str>;

    /// Returns the FIRST set of the rule with the given name, or `None` if the
    /// rule does not exist. This is the set of token types that can start the rule.
    fn first_set_named(&self, rule_name: &str) -> Option<std::collections::HashSet<&'static str>>;

    /// Build a `CstTree` from a boxed token iterator (streaming mode).
    fn run_streaming_building(
        &self,
        iter: Box<dyn Iterator<Item = Token<'static>>>,
        ctx: &mut DiagnosticsContext,
        profile: &ResolvedProfile,
        recovery: &RecoveryConfig,
        source_id: SourceId,
    ) -> CstTree;

    /// Collect all `ParseEvent`s from a boxed token iterator (streaming mode).
    fn run_streaming_collecting(
        &self,
        iter: Box<dyn Iterator<Item = Token<'static>>>,
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
        tokens: &[Token<'_>],
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

        let mut parse_ctx = ParseContext::new(
            SliceTokenSource::new(tokens),
            ctx, profile, source_id,
        );
        let mut strategy = CallbackStrategy(emit);
        let mut recovery_steps = 0usize;

        let start_expr = &self.grammar.exprs[self.grammar.start_idx];
        let _ = eval(start_expr, &mut parse_ctx, &self.grammar, &mut strategy, None, &mut recovery_steps, recovery.max_recovery_skips);
    }

    fn run_building(
        &self,
        tokens: &[Token<'_>],
        ctx: &mut DiagnosticsContext,
        profile: &ResolvedProfile,
        recovery: &RecoveryConfig,
        source_id: SourceId,
    ) -> CstTree {
        let mut parse_ctx = ParseContext::new(
            SliceTokenSource::new(tokens),
            ctx, profile, source_id,
        );
        let mut strategy = CstBuildingStrategy::new(source_id);
        let mut recovery_steps = 0usize;
        let start_expr = &self.grammar.exprs[self.grammar.start_idx];
        let _ = eval(start_expr, &mut parse_ctx, &self.grammar, &mut strategy, None, &mut recovery_steps, recovery.max_recovery_skips);
        strategy.finish()
    }

    fn run_collecting(
        &self,
        tokens: &[Token<'_>],
        ctx: &mut DiagnosticsContext,
        profile: &ResolvedProfile,
        recovery: &RecoveryConfig,
        source_id: SourceId,
        capacity_hint: usize,
    ) -> Vec<ParseEvent> {
        let mut parse_ctx = ParseContext::new(
            SliceTokenSource::new(tokens),
            ctx, profile, source_id,
        );
        let mut strategy = EventCollectingStrategy::with_capacity(capacity_hint);
        let mut recovery_steps = 0usize;
        let start_expr = &self.grammar.exprs[self.grammar.start_idx];
        let _ = eval(start_expr, &mut parse_ctx, &self.grammar, &mut strategy, None, &mut recovery_steps, recovery.max_recovery_skips);
        strategy.finish()
    }

    fn rule_names(&self) -> Vec<String> {
        self.grammar.rule_names.iter().cloned().collect()
    }

    fn first_set_start(&self) -> std::collections::HashSet<&'static str> {
        self.grammar.first_set_of(self.grammar.start_idx, &mut std::collections::HashSet::new())
    }

    fn first_set_named(&self, rule_name: &str) -> Option<std::collections::HashSet<&'static str>> {
        let idx = self.grammar.rule_names.iter().position(|n| n == rule_name)?;
        Some(self.grammar.first_set_of(idx, &mut std::collections::HashSet::new()))
    }

    fn run_streaming_building(
        &self,
        iter: Box<dyn Iterator<Item = Token<'static>>>,
        ctx: &mut DiagnosticsContext,
        profile: &ResolvedProfile,
        recovery: &RecoveryConfig,
        source_id: SourceId,
    ) -> CstTree {
        let source = BufferedTokenSource::new(iter, 64);
        let mut parse_ctx = ParseContext::new(source, ctx, profile, source_id);
        let mut strategy = CstBuildingStrategy::new(source_id);
        let mut recovery_steps = 0usize;
        let start_expr = &self.grammar.exprs[self.grammar.start_idx];
        let _ = eval(start_expr, &mut parse_ctx, &self.grammar, &mut strategy, None, &mut recovery_steps, recovery.max_recovery_skips);
        strategy.finish()
    }

    fn run_streaming_collecting(
        &self,
        iter: Box<dyn Iterator<Item = Token<'static>>>,
        ctx: &mut DiagnosticsContext,
        profile: &ResolvedProfile,
        recovery: &RecoveryConfig,
        source_id: SourceId,
        capacity_hint: usize,
    ) -> Vec<ParseEvent> {
        let source = BufferedTokenSource::new(iter, 64);
        let mut parse_ctx = ParseContext::new(source, ctx, profile, source_id);
        let mut strategy = EventCollectingStrategy::with_capacity(capacity_hint);
        let mut recovery_steps = 0usize;
        let start_expr = &self.grammar.exprs[self.grammar.start_idx];
        let _ = eval(start_expr, &mut parse_ctx, &self.grammar, &mut strategy, None, &mut recovery_steps, recovery.max_recovery_skips);
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

    /// Replace the recovery configuration. Returns `self` for chaining.
    ///
    /// # Example
    /// ```rust,ignore
    /// let parser = grammar.compile(&profile)?.with_recovery(RecoveryConfig::fail_fast());
    /// ```
    pub fn with_recovery(mut self, config: RecoveryConfig) -> Self {
        self.recovery = config;
        self
    }

    /// Set the maximum number of tokens the engine may skip in a single recovery
    /// step. Returns `self` for chaining.
    pub fn with_max_recovery_skips(mut self, skips: usize) -> Self {
        self.recovery.max_recovery_skips = skips;
        self
    }

    /// Set the maximum total error count before the engine stops attempting
    /// recovery. `0` means no limit. Returns `self` for chaining.
    pub fn with_max_errors(mut self, max: usize) -> Self {
        self.recovery.max_errors = max;
        self
    }

    /// Parse a token stream and return a [`CstTree`].
    ///
    /// Uses `SourceId(0)`. For multi-file workspaces use [`Self::parse_tree_with_source`].
    pub fn parse_tree(&self, tokens: &[Token<'_>], ctx: &mut DiagnosticsContext) -> CstTree {
        self.parse_tree_with_source(tokens, ctx, SourceId(0))
    }

    /// Parse a token stream and return a [`CstTree`], tagging all spans with `source_id`.
    pub fn parse_tree_with_source(
        &self,
        tokens: &[Token<'_>],
        ctx: &mut DiagnosticsContext,
        source_id: SourceId,
    ) -> CstTree {
        self.parse_fn.run_building(tokens, ctx, &self.profile, &self.recovery, source_id)
    }

    /// Parse a token stream and return a flat `Vec<ParseEvent>`.
    ///
    /// Uses `SourceId(0)`. For multi-file workspaces use [`Self::parse_events_with_source`].
    pub fn parse_events(&self, tokens: &[Token<'_>], ctx: &mut DiagnosticsContext) -> Vec<ParseEvent> {
        self.parse_events_with_source(tokens, ctx, SourceId(0))
    }

    /// Parse a token stream and return a flat `Vec<ParseEvent>`, tagging all spans with `source_id`.
    pub fn parse_events_with_source(
        &self,
        tokens: &[Token<'_>],
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
        tokens: &[Token<'_>],
        ctx: &mut DiagnosticsContext,
        strategy: S,
    ) -> S::Output {
        self.parse_with_strategy_and_source(tokens, ctx, strategy, SourceId(0))
    }

    /// Parse using a custom [`ParseStrategy`], tagging all spans with `source_id`.
    pub fn parse_with_strategy_and_source<S: ParseStrategy>(
        &self,
        tokens: &[Token<'_>],
        ctx: &mut DiagnosticsContext,
        mut strategy: S,
        source_id: SourceId,
    ) -> S::Output {
        self.parse_fn.run(tokens, ctx, &self.profile, &self.recovery, source_id,
            &mut |event| strategy.on_event(event));
        strategy.finish()
    }

    /// Parse a streaming iterator of `'static` tokens and return a [`CstTree`].
    ///
    /// Tokens are consumed lazily via a [`BufferedTokenSource`], keeping memory
    /// usage proportional to the active backtracking window instead of the full
    /// token stream. Uses `SourceId(0)`.
    ///
    /// # Note on lifetimes
    /// Streaming tokens must be `'static` because the iterator is boxed for
    /// object-safety. For source-borrowed tokens use [`Self::parse_tree`].
    pub fn parse_streaming(
        &self,
        iter: impl Iterator<Item = Token<'static>> + 'static,
        ctx: &mut DiagnosticsContext,
    ) -> CstTree {
        self.parse_fn.run_streaming_building(
            Box::new(iter), ctx, &self.profile, &self.recovery, SourceId(0),
        )
    }

    /// Parse a streaming iterator of `'static` tokens and return a flat
    /// `Vec<ParseEvent>`.
    pub fn parse_streaming_events(
        &self,
        iter: impl Iterator<Item = Token<'static>> + 'static,
        ctx: &mut DiagnosticsContext,
    ) -> Vec<ParseEvent> {
        self.parse_fn.run_streaming_collecting(
            Box::new(iter), ctx, &self.profile, &self.recovery, SourceId(0), 64,
        )
    }

    /// Build a stateful [`IncrementalParser`] backed by this compiled grammar.
    pub fn incremental(&self) -> IncrementalParser<'_> {
        IncrementalParser { compiled: self, cached_tree: None }
    }

    // ── Grammar introspection ─────────────────────────────────────────────────

    /// Returns the names of all rules in the grammar (in definition order).
    ///
    /// Useful for grammar testing ("does this rule exist?") and tooling
    /// (document-symbol providers, grammar visualisers).
    pub fn rule_names(&self) -> Vec<String> {
        self.parse_fn.rule_names()
    }

    /// Returns the FIRST set of the start rule — the token types that can
    /// legitimately begin a top-level parse.
    ///
    /// **Note**: For PEG grammars the FIRST set is an *approximation*. In
    /// particular, `Opt`/`Repeat0` rules are always considered nullable which
    /// can produce a superset of the true FIRST set.
    pub fn first_set_start(&self) -> std::collections::HashSet<&'static str> {
        self.parse_fn.first_set_start()
    }

    /// Returns the FIRST set of the named rule, or `None` if the rule does
    /// not exist in the grammar.
    ///
    /// See [`Self::first_set_start`] for the caveats.
    pub fn first_set_of(&self, rule_name: &str) -> Option<std::collections::HashSet<&'static str>> {
        self.parse_fn.first_set_named(rule_name)
    }
}

// ── IncrementalParser ─────────────────────────────────────────────────────────

/// A thin wrapper around a [`CompiledParser`] that caches the most-recently
/// produced [`CstTree`] and exposes a `reparse` API for incremental workflows.
pub struct IncrementalParser<'compiled> {
    compiled: &'compiled CompiledParser,
    cached_tree: Option<CstTree>,
}

impl<'compiled> IncrementalParser<'compiled> {
    /// Performs the initial full parse and stores the result in the cache.
    pub fn initial_parse(
        &mut self,
        tokens: &[Token<'_>],
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
        tokens: &[Token<'_>],
        _edits: &[TextEdit],
        ctx: &mut DiagnosticsContext,
    ) -> &CstTree {
        self.initial_parse(tokens, ctx)
    }

    /// Clears the cached tree, freeing its memory.
    pub fn invalidate(&mut self) {
        self.cached_tree = None;
    }

    /// Returns the cached tree from the most recent parse, if any.
    pub fn cached_tree(&self) -> Option<&CstTree> {
        self.cached_tree.as_ref()
    }
}

// ── TextEdit ──────────────────────────────────────────────────────────────────

/// A description of a text change applied to a source file.
///
/// Used with [`IncrementalParser::reparse`] to communicate edit regions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEdit {
    /// The byte-offset range of the text that was removed or replaced.
    pub range: std::ops::Range<usize>,
    /// The text that replaces the range (empty string for a deletion).
    pub replacement: String,
}

impl TextEdit {
    /// Creates a zero-width insertion at byte offset `at`.
    pub fn insert(at: usize, text: impl Into<String>) -> Self {
        TextEdit { range: at..at, replacement: text.into() }
    }
    /// Creates a deletion over `range`.
    pub fn delete(range: std::ops::Range<usize>) -> Self {
        TextEdit { range, replacement: String::new() }
    }
    /// Creates a replacement over `range`.
    pub fn replace(range: std::ops::Range<usize>, text: impl Into<String>) -> Self {
        TextEdit { range, replacement: text.into() }
    }
}

// ── Prelude ───────────────────────────────────────────────────────────────────

/// Convenience re-exports for the most commonly used grammar, CST, and profile types.
///
/// A typical grammar module can `use rb_parser::prelude::*` to bring everything needed
/// into scope without explicit qualified imports.
pub mod prelude {
    pub use crate::grammar::{
        GrammarRule, RuleId, RecoveryLandmarks, GrammarError,
        tok, tok_sub, ref_, seq2, alt2, repeat0, repeat1, opt, cut,
        node, field, between, list, list1, pratt, grammar,
        PrattBuilder, GrammarBuilder, Grammar,
    };
    pub use crate::cst::SyntaxKind;
    pub use crate::profiles::{ResolvedProfile, ProfileMode, profile_guard, profile, ProfileCatalog};
    pub use crate::{seq, one_of, any_of};
}
