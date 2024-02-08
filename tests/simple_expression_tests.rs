extern crate rb_tokenizer;

use rb_tokenizer::Tokenizer;

fn get_tokenizer() -> Tokenizer {
    let mut tokenizer = Tokenizer::new();

    tokenizer.add_regex_rule(r"^\d+", "Number", None);
    tokenizer.add_regex_rule(r"^[a-zA-Z_][a-zA-Z0-9_]*", "Identifier", None);
    tokenizer.add_symbol_rule("(", "Operator", Some("OpenParen"));
    tokenizer.add_symbol_rule(")", "Operator", Some("CloseParen"));

    // Operators
    tokenizer.add_symbol_rule("+", "Operator", Some("Plus"));
    tokenizer.add_symbol_rule("-", "Operator", Some("Minus"));
    tokenizer.add_symbol_rule("*", "Operator", Some("Multiply"));
    tokenizer.add_symbol_rule("/", "Operator", Some("Divide"));

    tokenizer
}

#[cfg(test)]
mod tests {
    use crate::get_tokenizer;

    #[test]
    fn it_works() {
        let tokenizer = get_tokenizer();
        let result = tokenizer.tokenize(r"2 + 2 * (3 + 4) - 5");
        println!("{:?}", result);
    }
}
