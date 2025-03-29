use rb_tokenizer::tokens::Token;
// parser_framework.rs

trait AstNode {}
struct EmptyNode;
impl AstNode for EmptyNode {}

#[derive(Debug)]
pub struct ParserError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

pub trait ParseRule {
    type Output;
    fn parse(
        &self,
        tokens: &[Token],
        position: usize,
    ) -> Result<Option<(Self::Output, usize)>, ParserError>;
}

// Example of a utility function to match specific tokens
pub fn match_token(
    expected_value: &'static str,
) -> Box<dyn Fn(&[Token], usize) -> Result<Option<(Token, usize)>, ParserError>> {
    Box::new(move |tokens, position| {
        tokens
            .get(position)
            .and_then(|token| {
                if token.value == expected_value {
                    Some(token)
                } else {
                    None
                }
            })
            .map_or(
                Err(ParserError {
                    message: format!(
                        "Expected '{}', found {:?}",
                        expected_value,
                        tokens.get(position).map(|t| &t.value)
                    ),
                    line: tokens.get(position).map_or(0, |t| t.line),
                    column: tokens.get(position).map_or(0, |t| t.column),
                }),
                |token| Ok(Some((token.clone(), position + 1))),
            )
    })
}

// Define other combinators like sequence, choice, repeat, etc., here

// Represents a parser that succeeds without consuming any input
fn empty() -> Box<dyn Fn(&[Token], usize) -> Result<Option<(Box<dyn AstNode>, usize)>, ParserError>>
{
    Box::new(|tokens, position| Ok(Some((Box::new(EmptyNode), position))))
}

// Represents a sequence of two parsers
fn sequence<F, G>(
    first: F,
    second: G,
) -> Box<dyn Fn(&[Token], usize) -> Result<Option<(Box<dyn AstNode>, usize)>, ParserError>>
where
    F: Fn(&[Token], usize) -> Result<Option<(Box<dyn AstNode>, usize)>, ParserError> + 'static,
    G: Fn(&[Token], usize) -> Result<Option<(Box<dyn AstNode>, usize)>, ParserError> + 'static,
{
    Box::new(move |tokens, position| {
        if let Ok(Some((node1, pos1))) = first(tokens, position) {
            if let Ok(Some((node2, pos2))) = second(tokens, pos1) {
                // Combine node1 and node2 into a new AST node as needed
                // For demonstration, assume a combined node type exists
                let combined_node = Box::new(CombinedNode {
                    parts: vec![*node1, *node2],
                });
                return Ok(Some((combined_node, pos2)));
            }
        }
        Err(ParserError {
            message: "Sequence parsing failed".into(),
            line: tokens.get(position).map_or(0, |t| t.line),
            column: tokens.get(position).map_or(0, |t| t.column),
        })
    })
}
