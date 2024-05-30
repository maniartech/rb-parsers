use crate::tokens::{Token, TokenizationError};

use super::regex_rule::RegexRule;
use super::symbol_rule::SymbolRule;
use super::{ClosureRule, Rule};

pub enum RuleType {
    Symbol(SymbolRule),
    Regex(RegexRule),
    Closure(ClosureRule),
    Rule(Box<dyn Rule>),
    Callback(Box<dyn CallbackRule>),
}

pub trait CallbackRule {
    fn process(&self, input: &str) -> Result<Option<Token>, TokenizationError>;
}

impl Rule for RuleType {
    fn process(&self, input: &str) -> Result<Option<Token>, TokenizationError> {
        match self {
            RuleType::Symbol(rule) => rule.process(input),
            RuleType::Regex(rule) => rule.process(input),
            RuleType::Closure(rule) => rule.process(input),
            RuleType::Rule(rule) => rule.process(input),
            RuleType::Callback(rule) => rule.process(input),
        }
    }
}
