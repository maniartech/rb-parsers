/// `Token` struct represents a token in a programming language.
#[derive(Debug, PartialEq, Clone)]
pub struct Token {
    pub token_type: &'static str,
    pub token_sub_type: Option<&'static str>,
    pub value: String,
    pub line: usize,
    pub column: usize,
}
