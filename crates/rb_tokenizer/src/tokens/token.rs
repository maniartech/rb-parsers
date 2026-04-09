use rb_common::spans::SourceSpan;

/// `Token` struct represents a token in a programming language.
#[derive(Debug, PartialEq, Clone)]
pub struct Token {
    pub token_type: &'static str,
    pub token_sub_type: Option<&'static str>,
    pub value: String,
    /// The byte-exact source location of this token.
    /// `span.start.byte_offset` is the byte index of the first character.
    /// `span.end.byte_offset` is the exclusive byte index after the last character.
    pub span: SourceSpan,
}

impl Token {
    /// Convenience accessor: 1-based display line.
    pub fn display_line(&self) -> usize {
        self.span.start.display_line()
    }

    /// Convenience accessor: 1-based display column.
    pub fn display_column(&self) -> usize {
        self.span.start.display_column()
    }
}
