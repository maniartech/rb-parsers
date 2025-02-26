extern crate rb_tokenizer;

use rb_tokenizer::{Tokenizer, TokenizerConfig};

fn get_tokenizer() -> Tokenizer {
    let config = TokenizerConfig {
        tokenize_whitespace: false,
        continue_on_error: true,
        error_tolerance_limit: 5,
    };
    let mut tokenizer = Tokenizer::with_config(config);

    // Structural characters
    tokenizer.add_symbol_rule("(", "Braces", Some("OpenParen"));
    tokenizer.add_symbol_rule(")", "Braces", Some("CloseParen"));
    tokenizer.add_symbol_rule("[", "Bracket", Some("OpenBracket"));
    tokenizer.add_symbol_rule("]", "Bracket", Some("CloseBracket"));
    tokenizer.add_symbol_rule(",", "Comma", None);

    // Arithmetic Operators
    tokenizer.add_symbol_rule("+", "Operator", Some("Plus"));
    tokenizer.add_symbol_rule("-", "Operator", Some("Minus"));
    tokenizer.add_symbol_rule("*", "Operator", Some("Multiply"));
    tokenizer.add_symbol_rule("/", "Operator", Some("Divide"));
    tokenizer.add_symbol_rule("%", "Operator", Some("Modulo"));

    // Comparison Operators
    tokenizer.add_symbol_rule("==", "Operator", Some("Equal"));
    tokenizer.add_symbol_rule("!=", "Operator", Some("NotEqual"));
    tokenizer.add_symbol_rule("<", "Operator", Some("LessThan"));
    tokenizer.add_symbol_rule("<=", "Operator", Some("LessThanOrEqual"));
    tokenizer.add_symbol_rule(">", "Operator", Some("GreaterThan"));
    tokenizer.add_symbol_rule(">=", "Operator", Some("GreaterThanOrEqual"));

    // Logical Operators
    tokenizer.add_symbol_rule("&&", "Operator", Some("And"));
    tokenizer.add_symbol_rule("||", "Operator", Some("Or"));
    tokenizer.add_symbol_rule("!", "Operator", Some("Not"));

    // Bitwise Operators
    tokenizer.add_symbol_rule("&", "Operator", Some("BitwiseAnd"));
    tokenizer.add_symbol_rule("^", "Operator", Some("BitwiseXor"));
    tokenizer.add_symbol_rule("~", "Operator", Some("BitwiseNot"));
    tokenizer.add_symbol_rule("<<", "Operator", Some("BitwiseLeftShift"));
    tokenizer.add_symbol_rule(">>", "Operator", Some("BitwiseRightShift"));

    // Literal Keywords
    tokenizer.add_regex_rule(r"^(true|false|null)\b", "Literal", None);

    // Raw String Literals in the form of: `string`, do not escape characters
    tokenizer.add_regex_rule(r#"^`([^`]|\\.)*`"#, "String", None);

    // String Literals in the form of: 'string'
    tokenizer.add_regex_rule(r#"^'([^'\\]|\\.)*'"#, "String", None);

    // String Literals in the form of: "string"
    tokenizer.add_regex_rule(r#"^"([^"\\]|\\.)*""#, "String", None);

    // Identifier
    tokenizer.add_regex_rule(r"^[a-zA-Z_][a-zA-Z0-9_]*", "Identifier", None);

    // Variable starts with $ and followed by letters, numbers, or underscores
    tokenizer.add_regex_rule(r"^\$[a-zA-Z0-9_]*", "Variable", None);

    // Named Pipes in the form of: |: with optional name |map: format
    tokenizer.add_regex_rule(r"^\|([a-zA-Z][a-zA-Z0-9_]*\:)?", "Pipe", None);

    tokenizer.add_symbol_rule("|", "Operator", Some("BitwiseOr"));

    // Real Numbers in the form of: 123, 123.456, 123.456e-78 with optional sign
    tokenizer.add_regex_rule(r"^-?\d+(\.\d+)?([eE][-+]?\d+)?", "Number", None);

    tokenizer
}

#[cfg(test)]
mod tests {
    use crate::get_tokenizer;

    #[test]
    fn it_works() {
        let tokenizer = get_tokenizer();
        let result = tokenizer.tokenize(r"[1, 2, 3, 4] |map: RAND() * $1 |filter: $1 % 2 == 0");
        println!("{:?}", result);
    }
    
    #[test]
    fn test_error_handling() {
        let tokenizer = get_tokenizer();
        // Introduce an invalid token (@ is not defined in our rules)
        let result = tokenizer.tokenize(r"[1, 2, @, 4]");
        assert!(result.is_err(), "Should return an error for invalid token");
        if let Err(errors) = result {
            println!("Tokenization errors: {:?}", errors);
            assert!(!errors.is_empty(), "Should contain at least one error");
        }
    }
}
