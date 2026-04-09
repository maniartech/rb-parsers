use rb_common::spans::{SourcePosition, SourceSpan};

use crate::cst::SyntaxKind;
use crate::engine::{ParseContext, ParseFailure, ParseOutcome};
use crate::events::ParseEvent;
use crate::profiles::{RecoveryConfig, ResolvedProfile, RuleProfileGuard};
use crate::strategy::ParseStrategy;

// ── RuleId ────────────────────────────────────────────────────────────────────

/// Marker trait for grammar rule identifier types.
///
/// Implement on a `#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]` enum.
pub trait RuleId: Copy + Clone + std::fmt::Debug + std::hash::Hash + Eq + Send + Sync + 'static {}

// ── RecoveryLandmarks ─────────────────────────────────────────────────────────

/// Token types recognized as safe re-entry points during error recovery.
#[derive(Debug, Clone)]
pub struct RecoveryLandmarks {
    token_types: Vec<&'static str>,
}

impl RecoveryLandmarks {
    pub fn from_token_types(types: &[&'static str]) -> Self {
        RecoveryLandmarks { token_types: types.to_vec() }
    }

    pub fn contains(&self, token_type: &str) -> bool {
        self.token_types.contains(&token_type)
    }
}

// ── GrammarError ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GrammarError {
    LeftRecursion { cycle: Vec<String> },
    UnreachableBranch { rule: String, branch_index: usize },
    ConflictingGuards { rule: String },
    NoStartRule,
    UnresolvedRef { rule_id: String },
    DuplicateRule { rule_id: String },
}

impl std::fmt::Display for GrammarError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GrammarError::LeftRecursion { cycle } =>
                write!(f, "left recursion: {}", cycle.join(" → ")),
            GrammarError::UnreachableBranch { rule, branch_index } =>
                write!(f, "unreachable branch {branch_index} in `{rule}`"),
            GrammarError::ConflictingGuards { rule } =>
                write!(f, "conflicting guards in `{rule}`"),
            GrammarError::NoStartRule =>
                write!(f, "no start rule registered"),
            GrammarError::UnresolvedRef { rule_id } =>
                write!(f, "unresolved rule ref: `{rule_id}`"),
            GrammarError::DuplicateRule { rule_id } =>
                write!(f, "duplicate rule: `{rule_id}`"),
        }
    }
}

impl std::error::Error for GrammarError {}

// ── Internal rule AST ─────────────────────────────────────────────────────────

pub(crate) enum RuleExpr<R: RuleId> {
    Tok { token_type: &'static str, token_sub_kind: Option<&'static str> },
    /// Build-time reference; replaced by `ResolvedRef` during `Grammar::compile()`.
    Ref(R),
    /// Runtime reference: an index into `CompiledGrammar::exprs`. Never present
    /// in a user-built expression tree — only produced by the compile step.
    ResolvedRef(usize),
    Seq(Box<RuleExpr<R>>, Box<RuleExpr<R>>),
    Alt(Box<RuleExpr<R>>, Box<RuleExpr<R>>),
    Repeat0(Box<RuleExpr<R>>),
    Repeat1(Box<RuleExpr<R>>),
    Opt(Box<RuleExpr<R>>),
    Cut,
    Node { kind: SyntaxKind, inner: Box<RuleExpr<R>> },
    Field { name: &'static str, inner: Box<RuleExpr<R>> },
    Between { open: Box<RuleExpr<R>>, body: Box<RuleExpr<R>>, close: Box<RuleExpr<R>> },
    List  { element: Box<RuleExpr<R>>, sep: Box<RuleExpr<R>> },
    List1 { element: Box<RuleExpr<R>>, sep: Box<RuleExpr<R>> },
    Guard { guard: RuleProfileGuard, inner: Box<RuleExpr<R>> },
    Recover { landmarks: RecoveryLandmarks, inner: Box<RuleExpr<R>> },
    Pratt(PrattSpec<R>),
}

// ── Pratt operator data ────────────────────────────────────────────────────────

pub(crate) struct PrattSpec<R: RuleId> {
    pub atom: Box<RuleExpr<R>>,
    pub ops: Vec<PrattOp<R>>,
}

pub(crate) struct PrattOp<R: RuleId> {
    pub token_type: &'static str,
    pub bp: u8,
    pub kind: PrattOpKind,
    pub node_kind: SyntaxKind,
    pub _ph: std::marker::PhantomData<R>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum PrattOpKind { Prefix, InfixLeft, InfixRight, Postfix }

// ── GrammarRule public trait ──────────────────────────────────────────────────

/// Trait implemented by every combinator return type.
///
/// `RuleExpr<R>` is `pub(crate)` intentionally — the bound `Into<RuleExpr<R>>`
/// makes `GrammarRule` effectively sealed: only types defined in this crate can
/// implement it, which is the desired invariant. External users compose grammars
/// through the free combinator functions rather than implementing the trait.
#[allow(private_bounds)]
pub trait GrammarRule<R: RuleId>: Sized + Into<RuleExpr<R>> {
    fn enabled_if(self, guard: RuleProfileGuard) -> GuardedRule<R> {
        GuardedRule { guard, inner: self.into() }
    }
    fn recover_to(self, landmarks: RecoveryLandmarks) -> RecoverRule<R> {
        RecoverRule { landmarks, inner: self.into() }
    }
}

/// Opaque wrapper produced by `.enabled_if()`. The `inner` field is intentionally
/// private; it is an implementation detail consumed inside this crate only.
pub struct GuardedRule<R: RuleId> { pub guard: RuleProfileGuard, pub(crate) inner: RuleExpr<R> }
/// Opaque wrapper produced by `.recover_to()`. The `inner` field is intentionally
/// private; it is an implementation detail consumed inside this crate only.
pub struct RecoverRule<R: RuleId> { pub landmarks: RecoveryLandmarks, pub(crate) inner: RuleExpr<R> }

impl<R: RuleId> From<GuardedRule<R>> for RuleExpr<R> {
    fn from(r: GuardedRule<R>) -> Self { RuleExpr::Guard { guard: r.guard, inner: Box::new(r.inner) } }
}
impl<R: RuleId> GrammarRule<R> for GuardedRule<R> {}

impl<R: RuleId> From<RecoverRule<R>> for RuleExpr<R> {
    fn from(r: RecoverRule<R>) -> Self { RuleExpr::Recover { landmarks: r.landmarks, inner: Box::new(r.inner) } }
}
impl<R: RuleId> GrammarRule<R> for RecoverRule<R> {}

// ── Newtype wrappers for each combinator return ───────────────────────────────

macro_rules! rule_wrapper {
    ($name:ident) => {
        pub struct $name<R: RuleId>(pub(crate) RuleExpr<R>);
        impl<R: RuleId> From<$name<R>> for RuleExpr<R> { fn from(r: $name<R>) -> Self { r.0 } }
        impl<R: RuleId> GrammarRule<R> for $name<R> {}
    };
}

rule_wrapper!(TokRule);
rule_wrapper!(RefRule);
rule_wrapper!(Seq2Rule);
rule_wrapper!(Alt2Rule);
rule_wrapper!(Repeat0Rule);
rule_wrapper!(Repeat1Rule);
rule_wrapper!(OptRule);
rule_wrapper!(CutRule);
rule_wrapper!(NodeRule);
rule_wrapper!(FieldRule);
rule_wrapper!(BetweenRule);
rule_wrapper!(ListRule);
rule_wrapper!(List1Rule);
rule_wrapper!(PrattRule);

// ── Free combinator functions ─────────────────────────────────────────────────

pub fn tok<R: RuleId>(token_type: &'static str) -> TokRule<R> {
    TokRule(RuleExpr::Tok { token_type, token_sub_kind: None })
}
pub fn tok_sub<R: RuleId>(token_type: &'static str, sub_kind: &'static str) -> TokRule<R> {
    TokRule(RuleExpr::Tok { token_type, token_sub_kind: Some(sub_kind) })
}
pub fn ref_<R: RuleId>(rule_id: R) -> RefRule<R> {
    RefRule(RuleExpr::Ref(rule_id))
}
pub fn seq2<R: RuleId>(a: impl GrammarRule<R>, b: impl GrammarRule<R>) -> Seq2Rule<R> {
    Seq2Rule(RuleExpr::Seq(Box::new(a.into()), Box::new(b.into())))
}
pub fn alt2<R: RuleId>(a: impl GrammarRule<R>, b: impl GrammarRule<R>) -> Alt2Rule<R> {
    Alt2Rule(RuleExpr::Alt(Box::new(a.into()), Box::new(b.into())))
}
pub fn repeat0<R: RuleId>(rule: impl GrammarRule<R>) -> Repeat0Rule<R> {
    Repeat0Rule(RuleExpr::Repeat0(Box::new(rule.into())))
}
pub fn repeat1<R: RuleId>(rule: impl GrammarRule<R>) -> Repeat1Rule<R> {
    Repeat1Rule(RuleExpr::Repeat1(Box::new(rule.into())))
}
pub fn opt<R: RuleId>(rule: impl GrammarRule<R>) -> OptRule<R> {
    OptRule(RuleExpr::Opt(Box::new(rule.into())))
}
pub fn cut<R: RuleId>() -> CutRule<R> { CutRule(RuleExpr::Cut) }
pub fn node<R: RuleId>(kind: SyntaxKind, rule: impl GrammarRule<R>) -> NodeRule<R> {
    NodeRule(RuleExpr::Node { kind, inner: Box::new(rule.into()) })
}
pub fn field<R: RuleId>(name: &'static str, rule: impl GrammarRule<R>) -> FieldRule<R> {
    FieldRule(RuleExpr::Field { name, inner: Box::new(rule.into()) })
}
pub fn between<R: RuleId>(
    open:  impl GrammarRule<R>,
    body:  impl GrammarRule<R>,
    close: impl GrammarRule<R>,
) -> BetweenRule<R> {
    BetweenRule(RuleExpr::Between {
        open: Box::new(open.into()),
        body: Box::new(body.into()),
        close: Box::new(close.into()),
    })
}
pub fn list<R: RuleId>(element: impl GrammarRule<R>, sep: impl GrammarRule<R>) -> ListRule<R> {
    ListRule(RuleExpr::List { element: Box::new(element.into()), sep: Box::new(sep.into()) })
}
pub fn list1<R: RuleId>(element: impl GrammarRule<R>, sep: impl GrammarRule<R>) -> List1Rule<R> {
    List1Rule(RuleExpr::List1 { element: Box::new(element.into()), sep: Box::new(sep.into()) })
}
pub fn pratt<R: RuleId>(atom: impl GrammarRule<R>) -> PrattBuilder<R> {
    PrattBuilder { atom: atom.into(), ops: Vec::new() }
}

// ── seq! / one_of! / any_of! macros ──────────────────────────────────────────

#[macro_export]
macro_rules! seq {
    ($e:expr) => { $e };
    ($first:expr, $($rest:expr),+) => {
        $crate::grammar::seq2($first, $crate::seq!($($rest),+))
    };
}

#[macro_export]
macro_rules! one_of {
    ($e:expr) => { $e };
    ($first:expr, $($rest:expr),+) => {
        $crate::grammar::alt2($first, $crate::one_of!($($rest),+))
    };
}

#[macro_export]
macro_rules! any_of {
    ($($tok:expr),+) => {
        $crate::grammar::RecoveryLandmarks::from_token_types(&[$($tok),+])
    };
}

// ── PrattBuilder ──────────────────────────────────────────────────────────────

pub struct PrattBuilder<R: RuleId> {
    atom: RuleExpr<R>,
    ops: Vec<PrattOp<R>>,
}

impl<R: RuleId> PrattBuilder<R> {
    pub fn prefix(mut self, token_type: &'static str, bp: u8, node_kind: SyntaxKind) -> Self {
        self.ops.push(PrattOp { token_type, bp, kind: PrattOpKind::Prefix, node_kind, _ph: std::marker::PhantomData });
        self
    }
    pub fn infix_left(mut self, token_type: &'static str, bp: u8, node_kind: SyntaxKind) -> Self {
        self.ops.push(PrattOp { token_type, bp, kind: PrattOpKind::InfixLeft, node_kind, _ph: std::marker::PhantomData });
        self
    }
    pub fn infix_right(mut self, token_type: &'static str, bp: u8, node_kind: SyntaxKind) -> Self {
        self.ops.push(PrattOp { token_type, bp, kind: PrattOpKind::InfixRight, node_kind, _ph: std::marker::PhantomData });
        self
    }
    pub fn postfix(mut self, token_type: &'static str, bp: u8, node_kind: SyntaxKind) -> Self {
        self.ops.push(PrattOp { token_type, bp, kind: PrattOpKind::Postfix, node_kind, _ph: std::marker::PhantomData });
        self
    }
    pub fn finish(self) -> PrattRule<R> {
        PrattRule(RuleExpr::Pratt(PrattSpec { atom: Box::new(self.atom), ops: self.ops }))
    }
}

// ── GrammarBuilder / Grammar<R> ───────────────────────────────────────────────

pub struct GrammarBuilder<R: RuleId> {
    rules: indexmap::IndexMap<String, (R, RuleExpr<R>)>,
}

pub struct Grammar<R: RuleId> {
    pub(crate) rules: indexmap::IndexMap<String, (R, RuleExpr<R>)>,
    pub(crate) start_key: String,
}

pub fn grammar<R: RuleId>() -> GrammarBuilder<R> {
    GrammarBuilder { rules: indexmap::IndexMap::new() }
}

impl<R: RuleId> GrammarBuilder<R> {
    pub fn rule(mut self, rule_id: R, rule: impl GrammarRule<R>) -> Self {
        let key = format!("{:?}", rule_id);
        self.rules.insert(key, (rule_id, rule.into()));
        self
    }
    pub fn start(self, start_rule: R) -> Grammar<R> {
        let start_key = format!("{:?}", start_rule);
        Grammar { rules: self.rules, start_key }
    }
}

impl<R: RuleId> Grammar<R> {
    pub fn compile(
        self,
        profile: &ResolvedProfile,
    ) -> Result<crate::CompiledParser, GrammarError> {
        // ── 1. Start-rule existence ───────────────────────────────────────────
        if !self.rules.contains_key(&self.start_key) {
            return Err(GrammarError::NoStartRule);
        }

        // ── 2. Unresolved reference detection ────────────────────────────────
        // Walk every rule expression; any Ref(R) whose key is not registered is
        // a compile-time error rather than a silent runtime soft-failure.
        let registered: std::collections::HashSet<&str> =
            self.rules.keys().map(|s| s.as_str()).collect();
        for (_, (_, expr)) in &self.rules {
            collect_unresolved_refs(expr, &registered)?;
        }

        // ── 3. Left-recursion detection ───────────────────────────────────────
        self.check_left_recursion()?;

        // ── 4. Build index map: rule key → position in flat Vec ───────────────
        let index_map: std::collections::HashMap<String, usize> = self
            .rules
            .keys()
            .enumerate()
            .map(|(i, k)| (k.clone(), i))
            .collect();
        let start_idx = *index_map.get(&self.start_key).unwrap(); // guaranteed by step 1

        // ── 5. Resolve all Ref(R) → ResolvedRef(usize) ───────────────────────
        let exprs: Vec<RuleExpr<R>> = self
            .rules
            .into_values()
            .map(|(_, expr)| resolve_refs(expr, &index_map))
            .collect::<Result<_, _>>()?;

        Ok(crate::CompiledParser::from_grammar(
            CompiledGrammar { exprs, start_idx },
            profile.clone(),
            RecoveryConfig::default(),
        ))
    }

    fn check_left_recursion(&self) -> Result<(), GrammarError> {
        fn first_refs<R: RuleId>(expr: &RuleExpr<R>) -> Vec<String> {
            match expr {
                RuleExpr::Ref(id) => vec![format!("{:?}", id)],
                // ResolvedRef only appears after compilation — check_left_recursion runs before that.
                RuleExpr::ResolvedRef(_) => vec![],
                RuleExpr::Seq(a, _) => first_refs(a),
                RuleExpr::Alt(a, b) => { let mut v = first_refs(a); v.extend(first_refs(b)); v }
                RuleExpr::Repeat0(i) | RuleExpr::Repeat1(i) | RuleExpr::Opt(i)
                | RuleExpr::Node { inner: i, .. } | RuleExpr::Field { inner: i, .. }
                | RuleExpr::Guard { inner: i, .. } | RuleExpr::Recover { inner: i, .. } => first_refs(i),
                RuleExpr::Between { open, .. } => first_refs(open),
                RuleExpr::List { element, .. } | RuleExpr::List1 { element, .. } => first_refs(element),
                RuleExpr::Pratt(p) => first_refs(&p.atom),
                RuleExpr::Tok { .. } | RuleExpr::Cut => vec![],
            }
        }

        for start_key in self.rules.keys() {
            let _stack: Vec<String> = vec![start_key.clone()];
            let mut visiting: Vec<String> = Vec::new();

            fn dfs<R: RuleId>(
                start: &str,
                current: &str,
                rules: &indexmap::IndexMap<String, (R, RuleExpr<R>)>,
                visiting: &mut Vec<String>,
                seen: &mut std::collections::HashSet<String>,
            ) -> Option<Vec<String>> {
                if let Some((_, expr)) = rules.get(current) {
                    for next in first_refs(expr) {
                        if next == start {
                            let mut cycle = visiting.clone();
                            cycle.push(next);
                            return Some(cycle);
                        }
                        if !seen.contains(&next) {
                            seen.insert(next.clone());
                            visiting.push(next.clone());
                            if let Some(c) = dfs(start, &next, rules, visiting, seen) {
                                return Some(c);
                            }
                            visiting.pop();
                        }
                    }
                }
                None
            }

            let mut seen = std::collections::HashSet::new();
            seen.insert(start_key.clone());
            if let Some(cycle) = dfs(start_key, start_key, &self.rules, &mut visiting, &mut seen) {
                return Err(GrammarError::LeftRecursion { cycle });
            }
        }
        Ok(())
    }
}

// ── Ref resolution (build-time → compile-time) ───────────────────────────────

/// Walks `expr` looking for `Ref(R)` variants whose key is not in `registered`.
/// Called during `Grammar::compile()` before any other transformation.
fn collect_unresolved_refs<R: RuleId>(
    expr: &RuleExpr<R>,
    registered: &std::collections::HashSet<&str>,
) -> Result<(), GrammarError> {
    match expr {
        RuleExpr::Ref(rule_id) => {
            let key = format!("{:?}", rule_id);
            if !registered.contains(key.as_str()) {
                return Err(GrammarError::UnresolvedRef { rule_id: key });
            }
            Ok(())
        }
        RuleExpr::ResolvedRef(_) | RuleExpr::Tok { .. } | RuleExpr::Cut => Ok(()),
        RuleExpr::Seq(a, b) | RuleExpr::Alt(a, b) => {
            collect_unresolved_refs(a, registered)?;
            collect_unresolved_refs(b, registered)
        }
        RuleExpr::Repeat0(i) | RuleExpr::Repeat1(i) | RuleExpr::Opt(i)
        | RuleExpr::Node { inner: i, .. } | RuleExpr::Field { inner: i, .. }
        | RuleExpr::Guard { inner: i, .. } | RuleExpr::Recover { inner: i, .. } => {
            collect_unresolved_refs(i, registered)
        }
        RuleExpr::Between { open, body, close } => {
            collect_unresolved_refs(open, registered)?;
            collect_unresolved_refs(body, registered)?;
            collect_unresolved_refs(close, registered)
        }
        RuleExpr::List { element, sep } | RuleExpr::List1 { element, sep } => {
            collect_unresolved_refs(element, registered)?;
            collect_unresolved_refs(sep, registered)
        }
        RuleExpr::Pratt(spec) => collect_unresolved_refs(&spec.atom, registered),
    }
}

/// Replaces every `Ref(R)` in `expr` with `ResolvedRef(usize)` by looking up
/// the rule identifier in `index_map`. Called after all validation passes.
fn resolve_refs<R: RuleId>(
    expr: RuleExpr<R>,
    index_map: &std::collections::HashMap<String, usize>,
) -> Result<RuleExpr<R>, GrammarError> {
    match expr {
        RuleExpr::Ref(rule_id) => {
            let key = format!("{:?}", rule_id);
            match index_map.get(&key) {
                Some(&idx) => Ok(RuleExpr::ResolvedRef(idx)),
                None => Err(GrammarError::UnresolvedRef { rule_id: key }),
            }
        }
        // Already resolved or leaf — nothing to do.
        already @ (RuleExpr::ResolvedRef(_) | RuleExpr::Tok { .. } | RuleExpr::Cut) => Ok(already),
        RuleExpr::Seq(a, b) => Ok(RuleExpr::Seq(
            Box::new(resolve_refs(*a, index_map)?),
            Box::new(resolve_refs(*b, index_map)?),
        )),
        RuleExpr::Alt(a, b) => Ok(RuleExpr::Alt(
            Box::new(resolve_refs(*a, index_map)?),
            Box::new(resolve_refs(*b, index_map)?),
        )),
        RuleExpr::Repeat0(i)  => Ok(RuleExpr::Repeat0(Box::new(resolve_refs(*i, index_map)?))),
        RuleExpr::Repeat1(i)  => Ok(RuleExpr::Repeat1(Box::new(resolve_refs(*i, index_map)?))),
        RuleExpr::Opt(i)      => Ok(RuleExpr::Opt(Box::new(resolve_refs(*i, index_map)?))),
        RuleExpr::Node { kind, inner } =>
            Ok(RuleExpr::Node { kind, inner: Box::new(resolve_refs(*inner, index_map)?) }),
        RuleExpr::Field { name, inner } =>
            Ok(RuleExpr::Field { name, inner: Box::new(resolve_refs(*inner, index_map)?) }),
        RuleExpr::Between { open, body, close } => Ok(RuleExpr::Between {
            open:  Box::new(resolve_refs(*open,  index_map)?),
            body:  Box::new(resolve_refs(*body,  index_map)?),
            close: Box::new(resolve_refs(*close, index_map)?),
        }),
        RuleExpr::List { element, sep } => Ok(RuleExpr::List {
            element: Box::new(resolve_refs(*element, index_map)?),
            sep:     Box::new(resolve_refs(*sep,     index_map)?),
        }),
        RuleExpr::List1 { element, sep } => Ok(RuleExpr::List1 {
            element: Box::new(resolve_refs(*element, index_map)?),
            sep:     Box::new(resolve_refs(*sep,     index_map)?),
        }),
        RuleExpr::Guard { guard, inner } =>
            Ok(RuleExpr::Guard { guard, inner: Box::new(resolve_refs(*inner, index_map)?) }),
        RuleExpr::Recover { landmarks, inner } =>
            Ok(RuleExpr::Recover { landmarks, inner: Box::new(resolve_refs(*inner, index_map)?) }),
        RuleExpr::Pratt(spec) => {
            let atom = resolve_refs(*spec.atom, index_map)?;
            Ok(RuleExpr::Pratt(PrattSpec { atom: Box::new(atom), ops: spec.ops }))
        }
    }
}

// ── CompiledGrammar (internal) ────────────────────────────────────────────────

/// An immutable, fully-resolved grammar ready for repeated evaluation.
///
/// All `Ref(R)` variants have been replaced with `ResolvedRef(usize)` pointing
/// into `exprs`. Constructed exclusively by `Grammar::compile()`.
pub(crate) struct CompiledGrammar<R: RuleId> {
    /// Flat rule array. Index-addressed; no heap allocation during evaluation.
    pub exprs: Vec<RuleExpr<R>>,
    /// Index of the start rule in `exprs`.
    pub start_idx: usize,
}

// ── Execution engine ──────────────────────────────────────────────────────────

/// Evaluate a rule expression against the parse context.
///
/// The explicit `'g` lifetime ties `expr` and `grammar` to the same borrow,
/// so `&grammar.exprs[idx]` (a sub-borrow of `grammar`) can be passed
/// recursively alongside `grammar` itself — all as immutable `'g` borrows,
/// without any `unsafe` code.
pub(crate) fn eval<'g, R: RuleId, S: ParseStrategy>(
    expr: &'g RuleExpr<R>,
    ctx: &mut ParseContext<'_>,
    grammar: &'g CompiledGrammar<R>,
    strategy: &mut S,
    current_field: Option<&'static str>,
    recovery_steps: &mut usize,
    max_recovery_steps: usize,
) -> ParseOutcome<()> {
    use ParseOutcome::*;

    match expr {
        // ── tok ──────────────────────────────────────────────────────────────
        RuleExpr::Tok { token_type, token_sub_kind } => {
            match ctx.peek() {
                Some(tok) if {
                    tok.token_type == *token_type
                        && match token_sub_kind {
                            None => true,
                            Some(sub) => tok.token_sub_type == Some(*sub),
                        }
                } => {
                    let tok = ctx.advance().unwrap();
                    let span = tok.span;
                    strategy.on_event(ParseEvent::Token {
                        token_type: tok.token_type,
                        token_sub_kind: tok.token_sub_type,
                        span,
                        is_trivia: false,
                        field_name: current_field,
                    });
                    Success(())
                }
                _ => SoftFailure(ParseFailure::soft(ctx.location(), Some(token_type))),
            }
        }

        // ── ref_ (build-time only — must never appear here) ─────────────────
        RuleExpr::Ref(_) => {
            // Ref(R) is only valid in the build phase. Grammar::compile() replaces
            // every Ref with a ResolvedRef(usize). Reaching here means a RuleExpr
            // was constructed manually and never compiled — that is a programming error.
            panic!("BUG: unresolved Ref in compiled grammar — always call Grammar::compile() first");
        }

        // ── resolved_ref (index into CompiledGrammar::exprs) ──────────────────
        RuleExpr::ResolvedRef(idx) => {
            // Both `sub_expr` and `grammar` share the lifetime `'g`, so the
            // borrow checker accepts passing them together without any unsafe code.
            let sub_expr: &'g RuleExpr<R> = &grammar.exprs[*idx];
            ctx.rule_depth += 1;
            let r = eval(sub_expr, ctx, grammar, strategy, current_field, recovery_steps, max_recovery_steps);
            ctx.rule_depth -= 1;
            r
        }

        // ── seq ───────────────────────────────────────────────────────────────
        RuleExpr::Seq(a, b) => {
            let save = ctx.cursor();
            match eval(a, ctx, grammar, strategy, current_field, recovery_steps, max_recovery_steps) {
                Success(()) => {
                    ctx.commit();
                    match eval(b, ctx, grammar, strategy, current_field, recovery_steps, max_recovery_steps) {
                        Success(()) => Success(()),
                        SoftFailure(f) => CommittedFailure(ParseFailure { committed: true, ..f }),
                        CommittedFailure(f) => CommittedFailure(f),
                    }
                }
                SoftFailure(f) => { ctx.reset_to(save); SoftFailure(f) }
                CommittedFailure(f) => CommittedFailure(f),
            }
        }

        // ── alt ───────────────────────────────────────────────────────────────
        RuleExpr::Alt(a, b) => {
            let save = ctx.cursor();
            match eval(a, ctx, grammar, strategy, current_field, recovery_steps, max_recovery_steps) {
                Success(()) => Success(()),
                CommittedFailure(f) => CommittedFailure(f),
                SoftFailure(_) => { ctx.reset_to(save); eval(b, ctx, grammar, strategy, current_field, recovery_steps, max_recovery_steps) }
            }
        }

        // ── repeat0 ───────────────────────────────────────────────────────────
        RuleExpr::Repeat0(inner) => {
            loop {
                let save = ctx.cursor();
                match eval(inner, ctx, grammar, strategy, current_field, recovery_steps, max_recovery_steps) {
                    Success(()) => {}
                    SoftFailure(_) => { ctx.reset_to(save); break; }
                    CommittedFailure(f) => return CommittedFailure(f),
                }
            }
            Success(())
        }

        // ── repeat1 ───────────────────────────────────────────────────────────
        RuleExpr::Repeat1(inner) => {
            let save = ctx.cursor();
            match eval(inner, ctx, grammar, strategy, current_field, recovery_steps, max_recovery_steps) {
                Success(()) => {}
                SoftFailure(f) => { ctx.reset_to(save); return SoftFailure(f); }
                CommittedFailure(f) => return CommittedFailure(f),
            }
            loop {
                let save = ctx.cursor();
                match eval(inner, ctx, grammar, strategy, current_field, recovery_steps, max_recovery_steps) {
                    Success(()) => {}
                    SoftFailure(_) => { ctx.reset_to(save); break; }
                    CommittedFailure(f) => return CommittedFailure(f),
                }
            }
            Success(())
        }

        // ── opt ───────────────────────────────────────────────────────────────
        RuleExpr::Opt(inner) => {
            let save = ctx.cursor();
            match eval(inner, ctx, grammar, strategy, current_field, recovery_steps, max_recovery_steps) {
                Success(()) => Success(()),
                SoftFailure(_) => { ctx.reset_to(save); Success(()) }
                CommittedFailure(f) => CommittedFailure(f),
            }
        }

        // ── cut ───────────────────────────────────────────────────────────────
        RuleExpr::Cut => { ctx.commit(); Success(()) }

        // ── node ──────────────────────────────────────────────────────────────
        RuleExpr::Node { kind, inner } => {
            let sid = ctx.source_id;
            let span_start = ctx.peek().map(|t| t.span.start).unwrap_or(SourcePosition::ZERO);
            strategy.on_event(ParseEvent::NodeStart { kind: *kind, span_start });
            let result = eval(inner, ctx, grammar, strategy, current_field, recovery_steps, max_recovery_steps);
            let span_end = ctx.tokens().get(ctx.cursor().saturating_sub(1))
                .map(|t| t.span.end)
                .unwrap_or(span_start);
            strategy.on_event(ParseEvent::NodeEnd { kind: *kind, span: SourceSpan::new(sid, span_start, span_end) });
            result
        }

        // ── field ─────────────────────────────────────────────────────────────
        RuleExpr::Field { name, inner } => {
            strategy.on_event(ParseEvent::FieldStart { name });
            let r = eval(inner, ctx, grammar, strategy, Some(name), recovery_steps, max_recovery_steps);
            strategy.on_event(ParseEvent::FieldEnd { name });
            r
        }

        // ── between ───────────────────────────────────────────────────────────
        RuleExpr::Between { open, body, close } => {
            let save = ctx.cursor();
            match eval(open, ctx, grammar, strategy, current_field, recovery_steps, max_recovery_steps) {
                Success(()) => {
                    ctx.commit();
                    // body failure is tolerated — we still try close
                    let _ = eval(body, ctx, grammar, strategy, current_field, recovery_steps, max_recovery_steps);
                    match eval(close, ctx, grammar, strategy, current_field, recovery_steps, max_recovery_steps) {
                        Success(()) => Success(()),
                        SoftFailure(f) => CommittedFailure(ParseFailure { committed: true, ..f }),
                        CommittedFailure(f) => CommittedFailure(f),
                    }
                }
                SoftFailure(f) => { ctx.reset_to(save); SoftFailure(f) }
                CommittedFailure(f) => CommittedFailure(f),
            }
        }

        // ── list ──────────────────────────────────────────────────────────────
        RuleExpr::List { element, sep } => {
            let save = ctx.cursor();
            match eval(element, ctx, grammar, strategy, current_field, recovery_steps, max_recovery_steps) {
                SoftFailure(_) => { ctx.reset_to(save); return Success(()); }
                CommittedFailure(f) => return CommittedFailure(f),
                Success(()) => {}
            }
            loop {
                let bs = ctx.cursor();
                match eval(sep, ctx, grammar, strategy, current_field, recovery_steps, max_recovery_steps) {
                    SoftFailure(_) => { ctx.reset_to(bs); break; }
                    CommittedFailure(f) => return CommittedFailure(f),
                    Success(()) => {
                        ctx.commit();
                        match eval(element, ctx, grammar, strategy, current_field, recovery_steps, max_recovery_steps) {
                            SoftFailure(f) => return CommittedFailure(ParseFailure { committed: true, ..f }),
                            CommittedFailure(f) => return CommittedFailure(f),
                            Success(()) => {}
                        }
                    }
                }
            }
            Success(())
        }

        // ── list1 ─────────────────────────────────────────────────────────────
        RuleExpr::List1 { element, sep } => {
            let save = ctx.cursor();
            match eval(element, ctx, grammar, strategy, current_field, recovery_steps, max_recovery_steps) {
                SoftFailure(f) => { ctx.reset_to(save); return SoftFailure(f); }
                CommittedFailure(f) => return CommittedFailure(f),
                Success(()) => {}
            }
            loop {
                let bs = ctx.cursor();
                match eval(sep, ctx, grammar, strategy, current_field, recovery_steps, max_recovery_steps) {
                    SoftFailure(_) => { ctx.reset_to(bs); break; }
                    CommittedFailure(f) => return CommittedFailure(f),
                    Success(()) => {
                        ctx.commit();
                        match eval(element, ctx, grammar, strategy, current_field, recovery_steps, max_recovery_steps) {
                            SoftFailure(f) => return CommittedFailure(ParseFailure { committed: true, ..f }),
                            CommittedFailure(f) => return CommittedFailure(f),
                            Success(()) => {}
                        }
                    }
                }
            }
            Success(())
        }

        // ── guard ─────────────────────────────────────────────────────────────
        RuleExpr::Guard { guard, inner } => {
            if guard.is_active(ctx.profile) {
                eval(inner, ctx, grammar, strategy, current_field, recovery_steps, max_recovery_steps)
            } else {
                SoftFailure(ParseFailure::soft(ctx.location(), None))
            }
        }

        // ── recover ───────────────────────────────────────────────────────────
        RuleExpr::Recover { landmarks, inner } => {
            match eval(inner, ctx, grammar, strategy, current_field, recovery_steps, max_recovery_steps) {
                CommittedFailure(f) => {
                    if *recovery_steps >= max_recovery_steps { return CommittedFailure(f); }
                    *recovery_steps += 1;
                    let mut skipped = 0usize;
                    while !ctx.at_eof() {
                        if ctx.peek().map(|t| landmarks.contains(t.token_type)).unwrap_or(false) {
                            break;
                        }
                        ctx.advance();
                        skipped += 1;
                    }
                    let landmark_type = ctx.peek().map(|t| t.token_type).unwrap_or("EOF");
                    strategy.on_event(ParseEvent::Recovery {
                        action: crate::engine::RecoveryAction::SkipTo {
                            landmark_token_type: landmark_type,
                            skipped_count: skipped,
                        },
                    });
                    Success(())
                }
                other => other,
            }
        }

        // ── pratt ─────────────────────────────────────────────────────────────
        RuleExpr::Pratt(spec) => {
            eval_pratt(spec, ctx, grammar, strategy, current_field, recovery_steps, max_recovery_steps, 0)
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn eval_pratt<'g, R: RuleId, S: ParseStrategy>(
    spec: &'g PrattSpec<R>,
    ctx: &mut ParseContext<'_>,
    grammar: &'g CompiledGrammar<R>,
    strategy: &mut S,
    current_field: Option<&'static str>,
    recovery_steps: &mut usize,
    max_recovery_steps: usize,
    min_bp: u8,
) -> ParseOutcome<()> {
    use ParseOutcome::*;

    // Prefix or atom
    let mut matched_prefix = false;
    for op in spec.ops.iter().filter(|o| o.kind == PrattOpKind::Prefix) {
        if ctx.peek().map(|t| t.token_type == op.token_type).unwrap_or(false) && op.bp >= min_bp {
            let sid = ctx.source_id;
            let tok = ctx.advance().unwrap();
            let span = tok.span;
            let span_start = tok.span.start;
            strategy.on_event(ParseEvent::NodeStart { kind: op.node_kind, span_start });
            strategy.on_event(ParseEvent::Token { token_type: tok.token_type, token_sub_kind: tok.token_sub_type, span, is_trivia: false, field_name: None });
            match eval_pratt(spec, ctx, grammar, strategy, current_field, recovery_steps, max_recovery_steps, op.bp) {
                Success(()) => {}
                f => {
                    strategy.on_event(ParseEvent::NodeEnd { kind: op.node_kind, span });
                    return f;
                }
            }
            let span_end = ctx.tokens().get(ctx.cursor().saturating_sub(1)).map(|t| t.span.end).unwrap_or(span_start);
            strategy.on_event(ParseEvent::NodeEnd { kind: op.node_kind, span: SourceSpan::new(sid, span_start, span_end) });
            matched_prefix = true;
            break;
        }
    }
    if !matched_prefix {
        match eval(&spec.atom, ctx, grammar, strategy, current_field, recovery_steps, max_recovery_steps) {
            Success(()) => {}
            other => return other,
        }
    }
    ctx.commit();

    loop {
        let before_op = ctx.cursor();
        let next_type = match ctx.peek().map(|t| t.token_type) { Some(t) => t, None => break };

        let mut handled = false;

        // Postfix
        for op in spec.ops.iter().filter(|o| o.kind == PrattOpKind::Postfix) {
            if op.token_type == next_type && op.bp > min_bp {
                let tok = ctx.advance().unwrap();
                let span = tok.span;
                let span_start = tok.span.start;
                strategy.on_event(ParseEvent::NodeStart { kind: op.node_kind, span_start });
                strategy.on_event(ParseEvent::Token { token_type: tok.token_type, token_sub_kind: tok.token_sub_type, span, is_trivia: false, field_name: None });
                strategy.on_event(ParseEvent::NodeEnd { kind: op.node_kind, span });
                handled = true;
                break;
            }
        }
        if handled { continue; }

        // Infix
        for op in spec.ops.iter().filter(|o| matches!(o.kind, PrattOpKind::InfixLeft | PrattOpKind::InfixRight)) {
            if op.token_type == next_type {
                let (l_bp, r_bp) = if op.kind == PrattOpKind::InfixLeft { (op.bp, op.bp + 1) } else { (op.bp, op.bp) };
                if l_bp <= min_bp { break; }
                let sid = ctx.source_id;
                let span_start = ctx.peek().map(|t| t.span.start).unwrap_or(SourcePosition::ZERO);
                let tok = ctx.advance().unwrap();
                let span = tok.span;
                strategy.on_event(ParseEvent::NodeStart { kind: op.node_kind, span_start });
                strategy.on_event(ParseEvent::Token { token_type: tok.token_type, token_sub_kind: tok.token_sub_type, span, is_trivia: false, field_name: None });
                match eval_pratt(spec, ctx, grammar, strategy, current_field, recovery_steps, max_recovery_steps, r_bp) {
                    Success(()) => {}
                    f => { strategy.on_event(ParseEvent::NodeEnd { kind: op.node_kind, span }); return f; }
                }
                let span_end = ctx.tokens().get(ctx.cursor().saturating_sub(1)).map(|t| t.span.end).unwrap_or(span_start);
                strategy.on_event(ParseEvent::NodeEnd { kind: op.node_kind, span: SourceSpan::new(sid, span_start, span_end) });
                handled = true;
                break;
            }
        }

        if !handled { ctx.reset_to(before_op); break; }
    }
    Success(())
}
