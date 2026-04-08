use rb_tokenizer::tokens::Token;

use crate::grammar::{Grammar, Symbol};

// ── Public output types ───────────────────────────────────────────────────────

/// A node in the concrete parse tree produced by the Earley parser.
#[derive(Debug, Clone)]
pub enum ParseNode {
    /// A terminal leaf node wrapping the matched [`Token`].
    Leaf(Token),

    /// An inner node produced by a grammar rule.
    Inner {
        /// The non-terminal name — the rule's `lhs`.
        name: &'static str,
        /// Optional label from the matched [`Rule`](crate::Rule), useful for
        /// distinguishing alternatives during AST construction.
        label: Option<&'static str>,
        /// Children in rule order.
        children: Vec<ParseNode>,
    },
}

/// The result of an [`EarleyParser::parse`] call.
#[derive(Debug)]
pub enum ParseResult {
    /// The token stream conforms to the grammar.
    Success {
        /// The concrete parse tree.
        tree: ParseNode,
        /// `true` if multiple complete derivations were found at the top level,
        /// indicating genuine syntactic ambiguity.
        ///
        /// **Note**: this flag detects top-level ambiguity only.  Deep structural
        /// ambiguity (multiple parse trees for an inner non-terminal) is not
        /// reported here; it is inherent to ambiguous grammars and all valid
        /// interpretations will tokenize successfully.
        ambiguous: bool,
    },
    /// The token stream does not conform to the grammar.
    Failure {
        /// The 0-based token index at which the parser could not advance further.
        position: usize,
        /// Human-readable description of what the parser expected at `position`.
        expected: Vec<String>,
    },
}

// ── Internal Earley types ─────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq)]
struct EarleyItem {
    rule_idx: usize,
    dot: usize,
    origin: usize,
}

/// Two variants of back-pointer stored with each completed/advanced item.
#[derive(Clone, Debug)]
struct BackPointer {
    /// Chart position and item index of the item *before* the dot was advanced.
    pred_chart: usize,
    pred_item: usize,
    /// For terminal advances: `usize::MAX` sentinel, token is `tokens[chart_pos - 1]`.
    /// For non-terminal completions: the chart position of the completing item.
    comp_chart: usize,
    /// For non-terminal completions: the item index of the completer.
    comp_item: usize,
}

#[derive(Clone, Debug)]
struct TrackedItem {
    item: EarleyItem,
    bp: Option<BackPointer>,
}

type Chart = Vec<Vec<TrackedItem>>;

// ── Chart helpers ─────────────────────────────────────────────────────────────

fn item_in_chart(set: &[TrackedItem], item: &EarleyItem) -> bool {
    set.iter().any(|ti| ti.item == *item)
}

// ── Earley algorithm steps ────────────────────────────────────────────────────

/// Process all items in `chart[pos]` — predict non-terminals and complete
/// finished items.  New items appended during the loop are also processed
/// (the while loop index handles this naturally).
fn build_set(grammar: &Grammar, chart: &mut Chart, pos: usize) {
    let mut i = 0;
    while i < chart[pos].len() {
        let ti = chart[pos][i].clone();
        let rule = &grammar.rules[ti.item.rule_idx];

        if ti.item.dot == rule.rhs.len() {
            // ── Completer ────────────────────────────────────────────────────
            let completed_lhs = rule.lhs;
            let origin = ti.item.origin;

            let active: Vec<(usize, EarleyItem)> = chart[origin]
                .iter()
                .enumerate()
                .filter(|(_, a)| {
                    let ar = &grammar.rules[a.item.rule_idx];
                    if a.item.dot < ar.rhs.len() {
                        if let Symbol::NonTerminal(lhs) = &ar.rhs[a.item.dot] {
                            return *lhs == completed_lhs;
                        }
                    }
                    false
                })
                .map(|(idx, a)| (idx, a.item.clone()))
                .collect();

            for (active_idx, active_item) in active {
                let new_item = EarleyItem {
                    rule_idx: active_item.rule_idx,
                    dot: active_item.dot + 1,
                    origin: active_item.origin,
                };
                if !item_in_chart(&chart[pos], &new_item) {
                    let bp = BackPointer {
                        pred_chart: origin,
                        pred_item: active_idx,
                        comp_chart: pos,
                        comp_item: i,
                    };
                    chart[pos].push(TrackedItem { item: new_item, bp: Some(bp) });
                }
            }
        } else {
            // ── Predictor ────────────────────────────────────────────────────
            if let Symbol::NonTerminal(lhs) = &grammar.rules[ti.item.rule_idx].rhs[ti.item.dot] {
                for rule_idx in grammar.rules_for(lhs) {
                    let new_item = EarleyItem { rule_idx, dot: 0, origin: pos };
                    if !item_in_chart(&chart[pos], &new_item) {
                        chart[pos].push(TrackedItem { item: new_item, bp: None });
                    }
                }
            }
        }

        i += 1;
    }
}

/// Scanner step: advance items from `chart[pos]` into `chart[pos + 1]` for
/// every item whose next symbol matches `token`.
fn scan_token(grammar: &Grammar, chart: &mut Chart, pos: usize, token: &Token) {
    let src = chart[pos].clone();
    for (idx, ti) in src.iter().enumerate() {
        let rule = &grammar.rules[ti.item.rule_idx];
        if ti.item.dot >= rule.rhs.len() {
            continue;
        }
        let matches = match &rule.rhs[ti.item.dot] {
            Symbol::Terminal(kind) => token.token_type == *kind,
            Symbol::TerminalSub(kind, sub) => {
                token.token_type == *kind && token.token_sub_type.as_deref() == Some(sub)
            }
            Symbol::NonTerminal(_) => false,
        };
        if matches {
            let new_item = EarleyItem {
                rule_idx: ti.item.rule_idx,
                dot: ti.item.dot + 1,
                origin: ti.item.origin,
            };
            if !item_in_chart(&chart[pos + 1], &new_item) {
                let bp = BackPointer {
                    pred_chart: pos,
                    pred_item: idx,
                    comp_chart: usize::MAX, // sentinel — terminal
                    comp_item: usize::MAX,
                };
                chart[pos + 1].push(TrackedItem { item: new_item, bp: Some(bp) });
            }
        }
    }
}

// ── Parse tree extraction ─────────────────────────────────────────────────────

/// Walk the back-pointer chain for the item at `chart[chart_pos][item_idx]` and
/// reconstruct the sequence of child [`ParseNode`]s it consumed.
fn extract_children(
    chart: &Chart,
    chart_pos: usize,
    item_idx: usize,
    grammar: &Grammar,
    tokens: &[Token],
) -> Vec<ParseNode> {
    let ti = &chart[chart_pos][item_idx];

    if ti.item.dot == 0 {
        return vec![];
    }

    let bp = match &ti.bp {
        None => return vec![],
        Some(bp) => bp.clone(),
    };

    if bp.comp_chart == usize::MAX {
        // ── Terminal: scanned token at position chart_pos - 1 ────────────────
        let leaf = ParseNode::Leaf(tokens[chart_pos - 1].clone());
        let mut children = extract_children(chart, bp.pred_chart, bp.pred_item, grammar, tokens);
        children.push(leaf);
        children
    } else {
        // ── Non-terminal: completed by another item ───────────────────────────
        let comp_ti = &chart[bp.comp_chart][bp.comp_item];
        let comp_rule = &grammar.rules[comp_ti.item.rule_idx];
        let comp_children =
            extract_children(chart, bp.comp_chart, bp.comp_item, grammar, tokens);
        let sub_tree = ParseNode::Inner {
            name: comp_rule.lhs,
            label: comp_rule.label,
            children: comp_children,
        };
        let mut children = extract_children(chart, bp.pred_chart, bp.pred_item, grammar, tokens);
        children.push(sub_tree);
        children
    }
}

// ── Public parser struct ──────────────────────────────────────────────────────

/// An Earley parser that accepts any context-free grammar (CFG), including
/// ambiguous ones.
///
/// Unlike PEG/recursive-descent parsers, the Earley algorithm:
/// - Handles **left recursion** natively.
/// - Handles **ambiguous grammars** (C++, natural language, etc.) — it detects
///   multiple parses and reports `ambiguous: true` in the result.
/// - Runs in **O(n³)** worst case (ambiguous), **O(n²)** for unambiguous CFGs,
///   and **O(n)** for LR(k) grammars.
///
/// # Usage
///
/// ```rust,ignore
/// use rb_parser::{EarleyParser, Grammar, Rule, Symbol};
/// use rb_tokenizer::Tokenizer;
///
/// let grammar = Grammar::new("expr", vec![
///     Rule::new("expr", vec![
///         Symbol::NonTerminal("expr"),
///         Symbol::Terminal("Plus"),
///         Symbol::NonTerminal("term"),
///     ]).with_label("add"),
///     Rule::new("expr", vec![Symbol::NonTerminal("term")]),
///     Rule::new("term", vec![Symbol::Terminal("Number")]).with_label("number"),
/// ]);
///
/// let mut tokenizer = Tokenizer::new();
/// tokenizer.add_regex_scanner(r"^\d+", "Number", None).unwrap();
/// tokenizer.add_symbol_scanner("+", "Plus", None);
///
/// let tokens = tokenizer.tokenize("1 + 2 + 3").unwrap();
/// let result = EarleyParser::new(&grammar).parse(&tokens);
/// ```
pub struct EarleyParser<'g> {
    grammar: &'g Grammar,
}

impl<'g> EarleyParser<'g> {
    pub fn new(grammar: &'g Grammar) -> Self {
        Self { grammar }
    }

    /// Parse a token stream produced by `rb_tokenizer`.
    pub fn parse(&self, tokens: &[Token]) -> ParseResult {
        let n = tokens.len();
        let mut chart: Chart = (0..=n).map(|_| Vec::new()).collect();

        // Seed the chart with all rules for the start symbol.
        for rule_idx in self.grammar.rules_for(self.grammar.start) {
            let item = EarleyItem { rule_idx, dot: 0, origin: 0 };
            if !item_in_chart(&chart[0], &item) {
                chart[0].push(TrackedItem { item, bp: None });
            }
        }
        build_set(self.grammar, &mut chart, 0);

        for i in 0..n {
            scan_token(self.grammar, &mut chart, i, &tokens[i]);
            if chart[i + 1].is_empty() {
                // No item could consume tokens[i] — parse fails here.
                break;
            }
            build_set(self.grammar, &mut chart, i + 1);
        }

        // Look for one or more completed start items spanning [0, n].
        let success_indices: Vec<usize> = chart[n]
            .iter()
            .enumerate()
            .filter(|(_, ti)| {
                let rule = &self.grammar.rules[ti.item.rule_idx];
                rule.lhs == self.grammar.start
                    && ti.item.dot == rule.rhs.len()
                    && ti.item.origin == 0
            })
            .map(|(i, _)| i)
            .collect();

        if success_indices.is_empty() {
            let position = (0..=n)
                .rev()
                .find(|&pos| !chart[pos].is_empty())
                .unwrap_or(0);
            ParseResult::Failure {
                position,
                expected: self.expected_at(&chart, position),
            }
        } else {
            let ambiguous = success_indices.len() > 1;
            let tree_idx = success_indices[0];
            let ti = &chart[n][tree_idx];
            let rule = &self.grammar.rules[ti.item.rule_idx];
            let children = extract_children(&chart, n, tree_idx, self.grammar, tokens);
            let tree = ParseNode::Inner {
                name: rule.lhs,
                label: rule.label,
                children,
            };
            ParseResult::Success { tree, ambiguous }
        }
    }

    fn expected_at(&self, chart: &Chart, pos: usize) -> Vec<String> {
        let mut expected = std::collections::BTreeSet::new();
        if let Some(items) = chart.get(pos) {
            for ti in items {
                let rule = &self.grammar.rules[ti.item.rule_idx];
                if ti.item.dot < rule.rhs.len() {
                    let desc = match &rule.rhs[ti.item.dot] {
                        Symbol::Terminal(k) => format!("token '{k}'"),
                        Symbol::TerminalSub(k, s) => format!("token '{k}:{s}'"),
                        Symbol::NonTerminal(n) => format!("<{n}>"),
                    };
                    expected.insert(desc);
                }
            }
        }
        expected.into_iter().collect()
    }
}
