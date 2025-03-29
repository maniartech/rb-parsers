struct Sequence {
    rules: Vec<Box<dyn ParseRule>>,
}

impl ParseRule for Sequence {
    fn parse(
        &self,
        tokens: &[Token],
        position: usize,
    ) -> Result<(Box<dyn AstNode>, usize), String> {
        let mut current_pos = position;
        for rule in &self.rules {
            match rule.parse(tokens, current_pos) {
                Ok((node, next_pos)) => {
                    // For simplicity, we're not accumulating the node here, but in a full implementation you would
                    current_pos = next_pos;
                }
                Err(e) => return Err(e),
            }
        }
        // Return success, probably with a specific AST node representing this sequence
        Ok((Box::new(YourSequenceNode {/* ... */}), current_pos))
    }
}
