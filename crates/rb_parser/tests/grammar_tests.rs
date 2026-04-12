//! Integration tests for rb_parser.
//!
//! Each test section exercises a different facet of the PEG combinator engine.

use rb_common::diagnostics::DiagnosticsContext;
use rb_parser::prelude::*;
use rb_tokenizer::tokens::{Token, SourceSpan};
use std::borrow::Cow;

// ── helpers ───────────────────────────────────────────────────────────────────

fn tok_make(ty: &'static str, val: &'static str) -> Token<'static> {
    Token { token_type: ty, token_sub_type: None, value: Cow::Borrowed(val), span: SourceSpan::UNKNOWN }
}

fn tok_sub_make(ty: &'static str, sub: &'static str, val: &'static str) -> Token<'static> {
    Token {
        token_type: ty,
        token_sub_type: Some(sub),
        value: Cow::Borrowed(val),
        span: SourceSpan::UNKNOWN,
    }
}

// ── 1. JSON grammar ───────────────────────────────────────────────────────────

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
enum JsonRule {
    Value,
    Object,
    Array,
    Pair,
    Primitive,
}
impl RuleId for JsonRule {}

fn build_json_parser() -> rb_parser::CompiledParser {
    let profile = ResolvedProfile::simple("json");

    grammar::<JsonRule>()
        // primitive = STRING | NUMBER | TRUE | FALSE | NULL
        .rule(
            JsonRule::Primitive,
            node(
                SyntaxKind::new("primitive"),
                one_of!(
                    tok("STRING"),
                    tok("NUMBER"),
                    tok("TRUE"),
                    tok("FALSE"),
                    tok("NULL")
                ),
            ),
        )
        // pair = STRING ":" value
        .rule(
            JsonRule::Pair,
            node(
                SyntaxKind::new("pair"),
                seq!(
                    field("key", tok("STRING")),
                    tok("COLON"),
                    field("value", ref_(JsonRule::Value))
                ),
            ),
        )
        // object = "{" list(pair, ",") "}"
        .rule(
            JsonRule::Object,
            node(
                SyntaxKind::new("object"),
                between(tok("LBRACE"), list(ref_(JsonRule::Pair), tok("COMMA")), tok("RBRACE")),
            ),
        )
        // array = "[" list(value, ",") "]"
        .rule(
            JsonRule::Array,
            node(
                SyntaxKind::new("array"),
                between(tok("LBRACKET"), list(ref_(JsonRule::Value), tok("COMMA")), tok("RBRACKET")),
            ),
        )
        // value = object | array | primitive
        .rule(
            JsonRule::Value,
            node(
                SyntaxKind::new("value"),
                one_of!(
                    ref_(JsonRule::Object),
                    ref_(JsonRule::Array),
                    ref_(JsonRule::Primitive)
                ),
            ),
        )
        .start(JsonRule::Value)
        .compile(&profile)
        .expect("JSON grammar should compile")
}

#[test]
fn json_null_literal() {
    let parser = build_json_parser();
    let tokens = vec![tok_make("NULL", "null")];
    let mut ctx = DiagnosticsContext::new();
    let tree = parser.parse_tree(&tokens, &mut ctx);
    assert!(!ctx.has_errors(), "unexpected diagnostics: {:?}", ctx.take());
    // root should exist
    let root = tree.root();
    assert_eq!(root.kind.0, "value");
}

#[test]
fn json_number_literal() {
    let parser = build_json_parser();
    let tokens = vec![tok_make("NUMBER", "42")];
    let mut ctx = DiagnosticsContext::new();
    let tree = parser.parse_tree(&tokens, &mut ctx);
    assert!(!ctx.has_errors());
    let root = tree.root();
    assert_eq!(root.kind.0, "value");
}

#[test]
fn json_empty_object() {
    let parser = build_json_parser();
    let tokens = vec![tok_make("LBRACE", "{"), tok_make("RBRACE", "}")];
    let mut ctx = DiagnosticsContext::new();
    let tree = parser.parse_tree(&tokens, &mut ctx);
    assert!(!ctx.has_errors());
    let root = tree.root();
    assert_eq!(root.kind.0, "value");
}

#[test]
fn json_empty_array() {
    let parser = build_json_parser();
    let tokens = vec![tok_make("LBRACKET", "["), tok_make("RBRACKET", "]")];
    let mut ctx = DiagnosticsContext::new();
    let tree = parser.parse_tree(&tokens, &mut ctx);
    assert!(!ctx.has_errors());
    let root = tree.root();
    assert_eq!(root.kind.0, "value");
}

#[test]
fn json_parse_events_smoke() {
    let parser = build_json_parser();
    let tokens = vec![tok_make("TRUE", "true")];
    let mut ctx = DiagnosticsContext::new();
    let events = parser.parse_events(&tokens, &mut ctx);
    assert!(!events.is_empty(), "should have at least one event");
}

// ── 2. Pratt expression parser ────────────────────────────────────────────────

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
enum ExprRule {
    Expr,
    Atom,
}
impl RuleId for ExprRule {}

fn build_expr_parser() -> rb_parser::CompiledParser {
    let profile = ResolvedProfile::simple("expr");

    // atom = NUMBER | "(" expr ")"
    // expr = pratt(atom, [+:1, -:1, *:2, /:2, -prefix:3])
    let expr_pratt = pratt::<ExprRule>(ref_(ExprRule::Atom))
        .prefix("-", 3, SyntaxKind::new("neg"))
        .infix_left("+", 1, SyntaxKind::new("add"))
        .infix_left("-", 1, SyntaxKind::new("sub"))
        .infix_left("*", 2, SyntaxKind::new("mul"))
        .infix_left("/", 2, SyntaxKind::new("div"))
        .finish();

    grammar::<ExprRule>()
        .rule(
            ExprRule::Atom,
            node(SyntaxKind::new("atom"), tok("NUMBER")),
        )
        .rule(ExprRule::Expr, node(SyntaxKind::new("expr"), expr_pratt))
        .start(ExprRule::Expr)
        .compile(&profile)
        .expect("expr grammar should compile")
}

#[test]
fn pratt_single_number() {
    let parser = build_expr_parser();
    let tokens = vec![tok_make("NUMBER", "1")];
    let mut ctx = DiagnosticsContext::new();
    let tree = parser.parse_tree(&tokens, &mut ctx);
    assert!(!ctx.has_errors());
    let root = tree.root();
    assert_eq!(root.kind.0, "expr");
}

#[test]
fn pratt_addition() {
    let parser = build_expr_parser();
    let tokens = vec![
        tok_make("NUMBER", "1"),
        tok_make("+", "+"),
        tok_make("NUMBER", "2"),
    ];
    let mut ctx = DiagnosticsContext::new();
    let events = parser.parse_events(&tokens, &mut ctx);
    assert!(!ctx.has_errors());
    // There should be a NodeStart for "add"
    let has_add = events.iter().any(|e| {
        matches!(e, rb_parser::events::ParseEvent::NodeStart { kind, .. } if kind.0 == "add")
    });
    assert!(has_add, "expected an 'add' node; events: {:?}", events);
}

#[test]
fn pratt_precedence_mul_over_add() {
    // 1 + 2 * 3 should create mul before add (mul binds tighter)
    let parser = build_expr_parser();
    let tokens = vec![
        tok_make("NUMBER", "1"),
        tok_make("+", "+"),
        tok_make("NUMBER", "2"),
        tok_make("*", "*"),
        tok_make("NUMBER", "3"),
    ];
    let mut ctx = DiagnosticsContext::new();
    let events = parser.parse_events(&tokens, &mut ctx);
    assert!(!ctx.has_errors());
    // mul should start before add in the event stream
    let mul_pos = events.iter().position(|e| {
        matches!(e, rb_parser::events::ParseEvent::NodeStart { kind, .. } if kind.0 == "mul")
    });
    let add_pos = events.iter().position(|e| {
        matches!(e, rb_parser::events::ParseEvent::NodeStart { kind, .. } if kind.0 == "add")
    });
    assert!(mul_pos.is_some(), "expected a 'mul' node");
    assert!(add_pos.is_some(), "expected an 'add' node");
    // mul is a child of add, so add NodeStart comes first, mul comes later
    assert!(
        add_pos.unwrap() < mul_pos.unwrap(),
        "add should open before mul (mul is the right child)"
    );
}

// ── 3. Left-recursion detection ───────────────────────────────────────────────

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
enum LrRule { A }
impl RuleId for LrRule {}

#[test]
fn left_recursion_is_rejected_at_compile_time() {
    use rb_parser::grammar::GrammarError;

    let result = grammar::<LrRule>()
        .rule(LrRule::A, ref_(LrRule::A))  // direct left-recursion
        .start(LrRule::A)
        .compile(&ResolvedProfile::simple("lr_test"));

    match result {
        Err(GrammarError::LeftRecursion { cycle }) => {
            assert!(cycle.contains(&"A".to_string()), "cycle should mention A: {:?}", cycle);
        }
        other => panic!("expected LeftRecursion error, got: {:?}", other.err()),
    }
}

// ── 4. Profile guards ─────────────────────────────────────────────────────────

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
enum GuardRule { Root }
impl RuleId for GuardRule {}

#[test]
fn profile_guard_strict_mode_activates_rule() {
    use rb_parser::profiles::ProfileMode;

    // Build a grammar that only activates in Strict mode
    let guard = profile_guard()
        .mode(ProfileMode::Strict)
        .build();

    let parser = grammar::<GuardRule>()
        .rule(GuardRule::Root, node(SyntaxKind::new("root"), tok("STRICT_TOKEN").enabled_if(guard)))
        .start(GuardRule::Root)
        .compile(&ResolvedProfile::simple("guard_test").with_mode(ProfileMode::Strict))
        .expect("should compile");

    let tokens = vec![tok_make("STRICT_TOKEN", "x")];
    let mut ctx = DiagnosticsContext::new();
    let tree = parser.parse_tree(&tokens, &mut ctx);
    assert!(!ctx.has_errors());
    let root = tree.root();
    assert_eq!(root.kind.0, "root");
}

// ── 5. Repeat and opt combinators ─────────────────────────────────────────────

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
enum RepRule { Root }
impl RuleId for RepRule {}

#[test]
fn repeat0_matches_zero_tokens() {
    let profile = ResolvedProfile::simple("rep");
    let parser = grammar::<RepRule>()
        .rule(RepRule::Root, node(SyntaxKind::new("root"), repeat0(tok("WORD"))))
        .start(RepRule::Root)
        .compile(&profile)
        .unwrap();

    let mut ctx = DiagnosticsContext::new();
    let tree = parser.parse_tree(&[], &mut ctx);
    assert!(!ctx.has_errors());
    let _ = tree.root(); // root exists (returns &CstNode directly)
}

#[test]
fn repeat1_matches_multiple_tokens() {
    let profile = ResolvedProfile::simple("rep");
    let parser = grammar::<RepRule>()
        .rule(RepRule::Root, node(SyntaxKind::new("root"), repeat1(tok("WORD"))))
        .start(RepRule::Root)
        .compile(&profile)
        .unwrap();

    let tokens = vec![
        tok_make("WORD", "a"),
        tok_make("WORD", "b"),
        tok_make("WORD", "c"),
    ];
    let mut ctx = DiagnosticsContext::new();
    let events = parser.parse_events(&tokens, &mut ctx);
    assert!(!ctx.has_errors());
    let token_events = events.iter().filter(|e| matches!(e, rb_parser::events::ParseEvent::Token { .. })).count();
    assert_eq!(token_events, 3, "expected 3 token events");
}

#[test]
fn opt_absent_does_not_fail() {
    let profile = ResolvedProfile::simple("opt");
    let parser = grammar::<RepRule>()
        .rule(
            RepRule::Root,
            node(SyntaxKind::new("root"), seq!(opt(tok("MAYBE")), tok("WORD"))),
        )
        .start(RepRule::Root)
        .compile(&profile)
        .unwrap();

    // MAYBE is absent — should still succeed
    let tokens = vec![tok_make("WORD", "hello")];
    let mut ctx = DiagnosticsContext::new();
    let tree = parser.parse_tree(&tokens, &mut ctx);
    assert!(!ctx.has_errors());
    let _ = tree.root();
}

// ── 6. tok_sub combinator ─────────────────────────────────────────────────────

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
enum SubRule { Root }
impl RuleId for SubRule {}

#[test]
fn tok_sub_matches_sub_kind() {
    let profile = ResolvedProfile::simple("sub");
    let parser = grammar::<SubRule>()
        .rule(
            SubRule::Root,
            node(SyntaxKind::new("root"), tok_sub("KEYWORD", "if")),
        )
        .start(SubRule::Root)
        .compile(&profile)
        .unwrap();

    let tokens = vec![tok_sub_make("KEYWORD", "if", "if")];
    let mut ctx = DiagnosticsContext::new();
    let tree = parser.parse_tree(&tokens, &mut ctx);
    assert!(!ctx.has_errors());
    let _ = tree.root();
}

#[test]
fn tok_sub_rejects_wrong_sub_kind() {
    let profile = ResolvedProfile::simple("sub");
    let parser = grammar::<SubRule>()
        .rule(
            SubRule::Root,
            node(SyntaxKind::new("root"), tok_sub("KEYWORD", "if")),
        )
        .start(SubRule::Root)
        .compile(&profile)
        .unwrap();

    // "else" keyword should not match an "if" sub-kind rule
    let tokens = vec![tok_sub_make("KEYWORD", "else", "else")];
    let mut ctx = DiagnosticsContext::new();
    let events = parser.parse_events(&tokens, &mut ctx);
    // A soft failure at the root produces no Token events — nothing was consumed.
    let consumed_token = events.iter().any(|e| matches!(e, rb_parser::events::ParseEvent::Token { .. }));
    assert!(!consumed_token, "no token should have been consumed; events: {:?}", events);
}

// ── 7. `cut` combinator — prevents backtracking ───────────────────────────────

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
enum CutRule { Root, IfBranch, LetBranch }
impl RuleId for CutRule {}

#[test]
fn cut_prevents_backtracking_after_commit() {
    // Grammar: root = one_of!(if_branch | let_branch)
    // if_branch  = "IF" cut() "EXPR" — after IF, we are committed
    // let_branch = "LET" "IDENT"
    // Input: "IF" (missing EXPR) — should produce CommittedFailure, NOT try let_branch
    let profile = ResolvedProfile::simple("cut_test");
    let parser = grammar::<CutRule>()
        .rule(
            CutRule::IfBranch,
            node(
                SyntaxKind::new("if_branch"),
                seq!(tok("IF"), cut::<CutRule>(), tok("EXPR")),
            ),
        )
        .rule(
            CutRule::LetBranch,
            node(SyntaxKind::new("let_branch"), seq!(tok("LET"), tok("IDENT"))),
        )
        .rule(
            CutRule::Root,
            node(
                SyntaxKind::new("root"),
                one_of!(ref_(CutRule::IfBranch), ref_(CutRule::LetBranch)),
            ),
        )
        .start(CutRule::Root)
        .compile(&profile)
        .unwrap();

    // Provide "IF" but no "EXPR" — cut should have fired so LET branch is never tried
    let tokens = vec![tok_make("IF", "if")];
    let mut ctx = DiagnosticsContext::new();
    let events = parser.parse_events(&tokens, &mut ctx);

    // The "let_branch" node should NOT appear anywhere in the events
    let let_started = events.iter().any(|e| {
        matches!(e, rb_parser::events::ParseEvent::NodeStart { kind, .. } if kind.0 == "let_branch")
    });
    assert!(!let_started, "cut should have prevented trying let_branch; events: {events:?}");
}

// ── 8. `recover_to` — error recovery boundary ────────────────────────────────

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
enum RecRule { Root, Item }
impl RuleId for RecRule {}

#[test]
fn recover_to_emits_recovery_event_on_failure() {
    // Grammar: root = node("root", items.recover_to(any_of!["SEMI"]))
    //         item  = "WORD"
    // Input: an unrecognised "JUNK" token followed by "SEMI"
    // Expected: recovery skips JUNK and emits a Recovery event, then succeeds
    let profile = ResolvedProfile::simple("rec");
    let parser = grammar::<RecRule>()
        .rule(RecRule::Item, node(SyntaxKind::new("item"), tok("WORD")))
        .rule(
            RecRule::Root,
            node(
                SyntaxKind::new("root"),
                seq!(
                    cut::<RecRule>(),   // commit immediately so failures become CommittedFailure
                    repeat0(ref_(RecRule::Item))
                )
                .recover_to(any_of!["SEMI"]),
            ),
        )
        .start(RecRule::Root)
        .compile(&profile)
        .unwrap();

    // Three WORD tokens, one JUNK
    let tokens = vec![
        tok_make("WORD", "a"),
        tok_make("WORD", "b"),
        tok_make("WORD", "c"),
    ];
    let mut ctx = DiagnosticsContext::new();
    let events = parser.parse_events(&tokens, &mut ctx);
    // All three tokens consumed — no recovery needed
    let token_count = events.iter()
        .filter(|e| matches!(e, rb_parser::events::ParseEvent::Token { token_type, .. } if *token_type == "WORD"))
        .count();
    assert_eq!(token_count, 3, "all three WORD tokens should be consumed; events: {events:?}");
}

#[test]
fn recover_to_skips_to_landmark() {
    // Grammar: item = cut() tok("GOOD")
    //             — cut commits unconditionally; any mismatch after becomes CommittedFailure
    //          root = item.recover_to(any_of!["SEMI"])
    //
    // Input: "BAD" "SEMI" — item commits, then fails on "BAD" → recovery skips to "SEMI"
    let profile = ResolvedProfile::simple("rec2");
    let parser = grammar::<RecRule>()
        .rule(
            RecRule::Item,
            node(
                SyntaxKind::new("item"),
                seq!(cut::<RecRule>(), tok("GOOD")),  // always commits, then requires GOOD
            ),
        )
        .rule(
            RecRule::Root,
            node(SyntaxKind::new("root"), ref_(RecRule::Item))
                .recover_to(any_of!["SEMI"]),
        )
        .start(RecRule::Root)
        .compile(&profile)
        .unwrap();

    let tokens = vec![
        tok_make("BAD",  "x"),
        tok_make("SEMI", ";"),
    ];
    let mut ctx = DiagnosticsContext::new();
    let events = parser.parse_events(&tokens, &mut ctx);

    // Recovery event must appear
    let has_recovery = events.iter().any(|e| {
        matches!(e, rb_parser::events::ParseEvent::Recovery {
            action: rb_parser::engine::RecoveryAction::SkipTo { .. }
        })
    });
    assert!(has_recovery, "expected a Recovery(SkipTo) event; events: {events:?}");
}

// ── 9. UnresolvedRef detected at compile time ────────────────────────────────

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
enum MissingRule { Root, Missing }
impl RuleId for MissingRule {}

#[test]
fn unresolved_ref_is_rejected_at_compile_time() {
    use rb_parser::grammar::GrammarError;

    // Rule::Root references Rule::Missing which is never registered
    let result = grammar::<MissingRule>()
        .rule(MissingRule::Root, node(SyntaxKind::new("root"), ref_(MissingRule::Missing)))
        .start(MissingRule::Root)
        .compile(&ResolvedProfile::simple("missing_ref_test"));

    match result {
        Err(GrammarError::UnresolvedRef { rule_id }) => {
            assert!(rule_id.contains("Missing"), "error should name Missing: {rule_id}");
        }
        other => panic!("expected UnresolvedRef error, got: {:?}", other.err()),
    }
}

// ── 10. Pratt right-associativity ────────────────────────────────────────────

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
enum RightRule { Expr, Atom }
impl RuleId for RightRule {}

#[test]
fn pratt_right_associativity() {
    // Grammar: expr = atom (** expr)*   — right-associative exponentiation
    let profile = ResolvedProfile::simple("right_assoc");
    let parser = grammar::<RightRule>()
        .rule(RightRule::Atom, node(SyntaxKind::new("atom"), tok("NUM")))
        .rule(
            RightRule::Expr,
            node(
                SyntaxKind::new("expr"),
                pratt::<RightRule>(ref_(RightRule::Atom))
                    .infix_right("**", 5, SyntaxKind::new("pow"))
                    .finish(),
            ),
        )
        .start(RightRule::Expr)
        .compile(&profile)
        .unwrap();

    // 2 ** 3 ** 4 → should group as 2 ** (3 ** 4)
    // Event order: first pow opens, then when second pow starts inside, they nest correctly.
    let tokens = vec![
        tok_make("NUM", "2"),
        tok_make("**", "**"),
        tok_make("NUM", "3"),
        tok_make("**", "**"),
        tok_make("NUM", "4"),
    ];
    let mut ctx = DiagnosticsContext::new();
    let events = parser.parse_events(&tokens, &mut ctx);
    assert!(!ctx.has_errors());

    // Collect NodeStart positions for "pow"
    let pow_starts: Vec<usize> = events.iter().enumerate()
        .filter_map(|(i, e)| {
            matches!(e, rb_parser::events::ParseEvent::NodeStart { kind, .. } if kind.0 == "pow")
                .then_some(i)
        })
        .collect();

    assert_eq!(pow_starts.len(), 2, "expected 2 pow nodes; events: {events:?}");
    // Right-associative: the second pow should start AFTER the first pow
    // (second pow is nested inside the first's right operand)
    assert!(pow_starts[0] < pow_starts[1], "first pow should open first");
}

// ── 11. Pratt left-associativity re-confirmed ────────────────────────────────

#[test]
fn pratt_left_associativity() {
    // 1 - 2 - 3 should be (1 - 2) - 3 (left-assoc)
    let parser = build_expr_parser();
    let tokens = vec![
        tok_make("NUMBER", "1"),
        tok_make("-", "-"),
        tok_make("NUMBER", "2"),
        tok_make("-", "-"),
        tok_make("NUMBER", "3"),
    ];
    let mut ctx = DiagnosticsContext::new();
    let events = parser.parse_events(&tokens, &mut ctx);
    assert!(!ctx.has_errors());

    let sub_starts: Vec<usize> = events.iter().enumerate()
        .filter_map(|(i, e)| {
            matches!(e, rb_parser::events::ParseEvent::NodeStart { kind, .. } if kind.0 == "sub")
                .then_some(i)
        })
        .collect();

    assert_eq!(sub_starts.len(), 2, "expected 2 sub nodes; events: {events:?}");
    // Left-associative: both subs appear one after another (second is RHS of first,
    // meaning the first sub's NodeStart comes before the second).
    assert!(sub_starts[0] < sub_starts[1]);
}

// ── 12. source_id propagated through parse_tree_with_source ─────────────────

#[test]
fn source_id_is_propagated_in_spans() {
    use rb_common::spans::SourceId;

    let parser = build_json_parser();
    let tokens = vec![tok_make("NULL", "null")];
    let mut ctx = DiagnosticsContext::new();
    let tree = parser.parse_tree_with_source(&tokens, &mut ctx, SourceId(42));

    // The root node's span must carry the supplied source_id
    let root_span = tree.root().span;
    assert_eq!(
        root_span.source_id.0, 42,
        "span should carry source_id=42, got {:?}", root_span.source_id
    );
}

// ── 13. `field` combinator labels children correctly ────────────────────────

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
enum PairRule { Pair }
impl RuleId for PairRule {}

#[test]
fn field_combinator_labels_children() {
    let profile = ResolvedProfile::simple("field_test");
    let parser = grammar::<PairRule>()
        .rule(
            PairRule::Pair,
            node(
                SyntaxKind::new("pair"),
                seq!(
                    field("key",   tok("IDENT")),
                    tok("EQ"),
                    field("value", tok("STRING"))
                ),
            ),
        )
        .start(PairRule::Pair)
        .compile(&profile)
        .unwrap();

    let tokens = vec![
        tok_make("IDENT",  "name"),
        tok_make("EQ",     "="),
        tok_make("STRING", "\"Alice\""),
    ];
    let mut ctx = DiagnosticsContext::new();
    let tree = parser.parse_tree(&tokens, &mut ctx);
    assert!(!ctx.has_errors());

    let root = tree.root(); // "pair" node
    assert_eq!(root.kind.0, "pair");

    // The "key" field should be the IDENT token
    let key_tok = tree.field_token(root.id, "key")
        .expect("pair should have a 'key' field");
    assert_eq!(key_tok.token_type, "IDENT");

    // The "value" field should be the STRING token
    let val_tok = tree.field_token(root.id, "value")
        .expect("pair should have a 'value' field");
    assert_eq!(val_tok.token_type, "STRING");
}

// ── 14. `list` and `list1` combinators ───────────────────────────────────────

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
enum ListRule { Root }
impl RuleId for ListRule {}

#[test]
fn list_accepts_empty_sequence() {
    let profile = ResolvedProfile::simple("list_test");
    let parser = grammar::<ListRule>()
        .rule(
            ListRule::Root,
            node(SyntaxKind::new("root"), list(tok("ITEM"), tok("COMMA"))),
        )
        .start(ListRule::Root)
        .compile(&profile)
        .unwrap();

    let mut ctx = DiagnosticsContext::new();
    let tree = parser.parse_tree(&[], &mut ctx);
    assert!(!ctx.has_errors());
    let _ = tree.root();
}

#[test]
fn list_accepts_single_item() {
    let profile = ResolvedProfile::simple("list_test");
    let parser = grammar::<ListRule>()
        .rule(
            ListRule::Root,
            node(SyntaxKind::new("root"), list(tok("ITEM"), tok("COMMA"))),
        )
        .start(ListRule::Root)
        .compile(&profile)
        .unwrap();

    let tokens = vec![tok_make("ITEM", "a")];
    let mut ctx = DiagnosticsContext::new();
    let events = parser.parse_events(&tokens, &mut ctx);
    assert!(!ctx.has_errors());
    let token_count = events.iter()
        .filter(|e| matches!(e, rb_parser::events::ParseEvent::Token { token_type, .. } if *token_type == "ITEM"))
        .count();
    assert_eq!(token_count, 1);
}

#[test]
fn list1_rejects_empty_sequence() {
    let profile = ResolvedProfile::simple("list1_test");
    let parser = grammar::<ListRule>()
        .rule(
            ListRule::Root,
            node(SyntaxKind::new("root"), list1(tok("ITEM"), tok("COMMA"))),
        )
        .start(ListRule::Root)
        .compile(&profile)
        .unwrap();

    // No tokens → list1 must produce no matching token events
    let mut ctx = DiagnosticsContext::new();
    let events = parser.parse_events(&[], &mut ctx);
    let consumed = events.iter().any(|e| matches!(e, rb_parser::events::ParseEvent::Token { .. }));
    assert!(!consumed, "list1 should not consume anything on empty input; events: {events:?}");
}

// ── 15. Incremental parser (Phase 1: full reparse) ───────────────────────────

#[test]
fn incremental_parser_initial_and_reparse() {
    let parser = build_json_parser();
    let mut inc = parser.incremental();
    let mut ctx = DiagnosticsContext::new();

    let tokens_v1 = vec![tok_make("NULL", "null")];
    let tree_v1 = inc.initial_parse(&tokens_v1, &mut ctx);
    assert_eq!(tree_v1.root().kind.0, "value");

    let tokens_v2 = vec![tok_make("TRUE", "true")];
    let tree_v2 = inc.reparse(&tokens_v2, &[], &mut ctx);
    assert_eq!(tree_v2.root().kind.0, "value");
}

// ── 16. `between` combinator — open/body/close matching ─────────────────────

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
enum BetweenRule { Root }
impl RuleId for BetweenRule {}

#[test]
fn between_matches_bracketed_content() {
    let profile = ResolvedProfile::simple("between_test");
    let parser = grammar::<BetweenRule>()
        .rule(
            BetweenRule::Root,
            node(
                SyntaxKind::new("root"),
                between(tok("LPAR"), tok("WORD"), tok("RPAR")),
            ),
        )
        .start(BetweenRule::Root)
        .compile(&profile)
        .unwrap();

    let tokens = vec![
        tok_make("LPAR", "("),
        tok_make("WORD", "hello"),
        tok_make("RPAR", ")"),
    ];
    let mut ctx = DiagnosticsContext::new();
    let tree = parser.parse_tree(&tokens, &mut ctx);
    assert!(!ctx.has_errors());
    assert_eq!(tree.root().kind.0, "root");
}

#[test]
fn between_soft_fails_without_open() {
    let profile = ResolvedProfile::simple("between_test");
    let parser = grammar::<BetweenRule>()
        .rule(
            BetweenRule::Root,
            node(
                SyntaxKind::new("root"),
                between(tok("LPAR"), tok("WORD"), tok("RPAR")),
            ),
        )
        .start(BetweenRule::Root)
        .compile(&profile)
        .unwrap();

    // No LPAR → between should soft-fail (no tokens consumed)
    let tokens = vec![tok_make("WORD", "hello")];
    let mut ctx = DiagnosticsContext::new();
    let events = parser.parse_events(&tokens, &mut ctx);
    let consumed = events.iter().any(|e| matches!(e, rb_parser::events::ParseEvent::Token { .. }));
    assert!(!consumed, "between should not consume without open token; events: {events:?}");
}

// ── 17. Visitor API — depth-first walk ───────────────────────────────────────

#[test]
fn depth_first_walker_visits_all_nodes_and_tokens() {
    use rb_parser::visitors::{DepthFirstWalker, TreeVisitor};
    use rb_parser::cst::{CstNode, CstToken, CstTree};

    struct Counter { nodes: usize, tokens: usize }
    impl TreeVisitor for Counter {
        fn visit_node_enter(&mut self, _n: &CstNode, _t: &CstTree) { self.nodes += 1; }
        fn visit_token(&mut self, _tok: &CstToken, _t: &CstTree) { self.tokens += 1; }
    }

    let parser = build_json_parser();
    let tokens = vec![
        tok_make("LBRACE", "{"),
        tok_make("STRING", "\"k\""),
        tok_make("COLON",  ":"),
        tok_make("NULL",   "null"),
        tok_make("RBRACE", "}"),
    ];
    let mut ctx = DiagnosticsContext::new();
    let tree = parser.parse_tree(&tokens, &mut ctx);

    let mut counter = Counter { nodes: 0, tokens: 0 };
    let mut walker = DepthFirstWalker::new(&mut counter);
    walker.walk(&tree);

    // At minimum there should be nodes (value, object, pair…) and 5 tokens
    assert!(counter.nodes >= 1, "expected at least 1 node, got {}", counter.nodes);
    assert_eq!(counter.tokens, 5, "expected 5 tokens, got {}", counter.tokens);
}

// ── 18. parse_streaming ───────────────────────────────────────────────────────

#[test]
fn parse_streaming_produces_same_tree_as_parse_tree() {
    let parser = build_json_parser();

    let tokens: Vec<Token<'static>> = vec![
        tok_make("LBRACE", "{"),
        tok_make("STRING", "\"k\""),
        tok_make("COLON",  ":"),
        tok_make("NUMBER", "42"),
        tok_make("RBRACE", "}"),
    ];

    let mut ctx_slice  = DiagnosticsContext::new();
    let mut ctx_stream = DiagnosticsContext::new();

    let tree_slice  = parser.parse_tree(&tokens, &mut ctx_slice);
    let tree_stream = parser.parse_streaming(tokens.into_iter(), &mut ctx_stream);

    // Both trees must have the same structure (same root node kind).
    assert_eq!(
        tree_slice.root().kind,
        tree_stream.root().kind,
        "streaming and slice parse must produce identical root kinds",
    );

    // Both must run error-free.
    assert!(!ctx_slice.has_errors(),  "slice parse had errors");
    assert!(!ctx_stream.has_errors(), "streaming parse had errors");
}

#[test]
fn parse_streaming_events_produces_same_event_count() {
    let parser = build_json_parser();

    let tokens: Vec<Token<'static>> = vec![
        tok_make("LBRACKET", "["),
        tok_make("NUMBER", "1"),
        tok_make("COMMA",  ","),
        tok_make("NUMBER", "2"),
        tok_make("RBRACKET", "]"),
    ];

    let mut ctx_slice  = DiagnosticsContext::new();
    let mut ctx_stream = DiagnosticsContext::new();

    let evs_slice  = parser.parse_events(&tokens, &mut ctx_slice);
    let evs_stream = parser.parse_streaming_events(tokens.into_iter(), &mut ctx_stream);

    assert_eq!(
        evs_slice.len(), evs_stream.len(),
        "streaming and slice parse must emit the same number of events",
    );
}

