/// A symbol appearing in a grammar rule's right-hand side.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Symbol {
    /// Matches a [`Token`](rb_tokenizer::tokens::Token) whose `token_type` equals
    /// the given string.
    Terminal(&'static str),

    /// Matches a [`Token`](rb_tokenizer::tokens::Token) whose `token_type` equals
    /// the first string **and** `token_sub_type` equals `Some(second string)`.
    TerminalSub(&'static str, &'static str),

    /// References a non-terminal — another group of rules in the grammar
    /// identified by name.
    NonTerminal(&'static str),
}

/// A single context-free production rule: `lhs → rhs₀ rhs₁ … rhsₙ`.
///
/// An empty `rhs` represents an epsilon (ε) production.
#[derive(Debug, Clone)]
pub struct Rule {
    /// The non-terminal this rule expands.
    pub lhs: &'static str,

    /// The right-hand side sequence.  May be empty (epsilon production).
    pub rhs: Vec<Symbol>,

    /// Optional label surfaced in the [`ParseNode`](crate::ParseNode) for this rule.
    /// Useful for distinguishing alternatives during AST construction.
    pub label: Option<&'static str>,
}

impl Rule {
    /// Create a rule with no label.
    pub fn new(lhs: &'static str, rhs: Vec<Symbol>) -> Self {
        Self { lhs, rhs, label: None }
    }

    /// Builder method to attach a label to this rule.
    pub fn with_label(mut self, label: &'static str) -> Self {
        self.label = Some(label);
        self
    }
}

/// A context-free grammar (CFG).
///
/// Supports the full class of CFGs — deterministic, non-deterministic, and
/// ambiguous grammars.  Use with [`EarleyParser`](crate::EarleyParser) which
/// handles all CFGs in O(n³) time (O(n²) for unambiguous, O(n) for most
/// practical grammars).
///
/// # Example — arithmetic expressions
///
/// ```rust
/// use rb_parser::{Grammar, Rule, Symbol};
///
/// let grammar = Grammar::new("expr", vec![
///     // expr → expr + term  (left-associative addition)
///     Rule::new("expr", vec![
///         Symbol::NonTerminal("expr"),
///         Symbol::Terminal("Plus"),
///         Symbol::NonTerminal("term"),
///     ]).with_label("add"),
///     // expr → term
///     Rule::new("expr", vec![Symbol::NonTerminal("term")]),
///     // term → Number
///     Rule::new("term", vec![Symbol::Terminal("Number")]).with_label("number"),
/// ]);
/// ```
pub struct Grammar {
    /// All production rules in the grammar.
    pub rules: Vec<Rule>,

    /// The start symbol.  The parser accepts input when this non-terminal
    /// spans the entire token stream.
    pub start: &'static str,
}

impl Grammar {
    pub fn new(start: &'static str, rules: Vec<Rule>) -> Self {
        Self { rules, start }
    }

    /// Return the indices of all rules whose `lhs` matches `name`.
    pub(crate) fn rules_for(&self, name: &str) -> Vec<usize> {
        self.rules
            .iter()
            .enumerate()
            .filter(|(_, r)| r.lhs == name)
            .map(|(i, _)| i)
            .collect()
    }
}
