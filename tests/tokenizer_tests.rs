extern crate rb_tokenizer;

use rb_tokenizer::{
    rules::{RegexRule, RuleType, SymbolRule},
    Tokenizer,
};

fn get_tokenizer() -> Tokenizer {
    let mut tokenizer = Tokenizer::new();

    tokenizer.add_rule(RuleType::Regex(RegexRule::new(r"^\d+", "Number", None)));

    tokenizer.add_rule(RuleType::Regex(RegexRule::new(
        r"^[a-zA-Z_][a-zA-Z0-9_]*",
        "Identifier",
        None,
    )));

    tokenizer.add_rule(RuleType::Symbol(SymbolRule::new(
        "(",
        "Operator",
        Some("OpenParen"),
    )));

    tokenizer.add_rule(RuleType::Symbol(SymbolRule::new(
        ")",
        "Operator",
        Some("CloseParen"),
    )));

    tokenizer.add_rule(RuleType::Symbol(SymbolRule::new(
        "+",
        "Operator",
        Some("Plus"),
    )));

    tokenizer
}

#[cfg(test)]
mod tests {
    use crate::get_tokenizer;

    #[test]
    fn it_works() {
        let tokenizer = get_tokenizer();
        let result = tokenizer.tokenize(r"ADD(2 +    2)");
        println!("{:?}", result);
    }
}
