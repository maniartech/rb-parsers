use rb_common::spans::SourceSpan;
use std::borrow::Cow;

/// A token produced by lexing source text.
///
/// The `'src` lifetime is tied to the original source `&str` passed to
/// [`Tokenizer::tokenize`](crate::tokenizers::Tokenizer::tokenize).
/// Non-transforming scanners (keywords, operators, identifiers, …) set
/// `value` to a [`Cow::Borrowed`] slice of that source, incurring **zero
/// allocation**. Scanners that decode escape sequences or otherwise transform
/// the matched text use [`Cow::Owned`].
#[derive(Debug, PartialEq, Clone)]
pub struct Token<'src> {
    pub token_type: &'static str,
    pub token_sub_type: Option<&'static str>,
    /// The lexed text. Borrows from the source input wherever possible.
    pub value: Cow<'src, str>,
    /// The byte-exact source location of this token.
    /// `span.start.byte_offset` is the byte index of the first character.
    /// `span.end.byte_offset` is the exclusive byte index after the last character.
    pub span: SourceSpan,
}

impl<'src> Token<'src> {
    /// Convenience accessor: 1-based display line.
    pub fn display_line(&self) -> usize {
        self.span.start.display_line()
    }

    /// Convenience accessor: 1-based display column.
    pub fn display_column(&self) -> usize {
        self.span.start.display_column()
    }
}
