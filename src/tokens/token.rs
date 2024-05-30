/// `Token` struct represents a token in a programming language.
#[derive(Debug, PartialEq, Clone)]
pub struct Token {
    /// `token_type` is a string that represents the type of the token.
    pub token_type: String,

    /// `token_sub_type` is an optional string that represents the subtype of the token.
    /// It is `None` if the token does not have a subtype. For example, a token of type
    /// `number` may have a subtype of `integer` or `float`. If the token is an operator,
    /// the subtype may represent the specific operator such as `arithmetic`, `logical`, etc.
    pub token_sub_type: Option<String>,

    /// `value` is a string that represents the actual value of the token.
    pub value: String,

    /// `line` is an unsigned integer that represents the line number in the source code where the token is found.
    pub line: usize,

    /// `column` is an unsigned integer that represents the column number in the source code where the token starts.
    pub column: usize,
}
