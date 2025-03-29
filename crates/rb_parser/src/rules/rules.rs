trait ParseRule {
    fn parse(&self, tokens: &[Token], position: usize)
        -> Result<(Box<dyn AstNode>, usize), String>;
}

struct Choice {
    rules: Vec<Box<dyn ParseRule>>,
}

// Implementations for these would follow the PEG logic, attempting to match each rule in sequence or choice, etc.
