# Syntax Tree and Materialization

## Objective

Define the default syntax-tree representation for `rb_parsers`, how higher-level structures such as ASTs are produced from it, and the performance constraints that must hold from day one.

This spec exists because tree-first parsing is only competitive if the tree representation is compact, source-aware, and compatible with later incremental reuse.

## Core Decisions

The current recommended direction is:

1. the first stable parse result should be a CST
2. the CST should be source-preserving enough for diagnostics, tooling, and future formatting or refactoring workflows
3. ASTs should be derived from the CST through lowering or materialization strategies rather than being the default parse product
4. the default CST must be performance-constrained from day one so tree-first parsing remains viable for the common case

## Why CST-First

A CST is the safest default foundation for the framework goals already established elsewhere.

It preserves:

- exact structural boundaries
- delimiter and recovery artifacts
- source spans and token relationships
- enough structure for rich diagnostics and tooling
- a stable base for incremental reuse

An AST is still important, but it should usually be the semantic or ergonomic layer built on top of this source-preserving structure.

## Performance Constraint

CST-first is only acceptable if the representation remains compact.

The framework must not implement the default CST in a way that:

- allocates one heap object per node without restraint
- duplicates source text into nodes and tokens
- eagerly builds both CST and AST on every parse
- stores trivia in bloated object graphs
- prevents efficient reuse of unchanged subtrees

The design goal is not "tree at any cost." The design goal is "tree by default, without losing competitiveness."

## Representation Requirements

The default tree representation should satisfy these constraints:

1. nodes and tokens should reference source by span, token id, or shared backing storage rather than owning duplicate text by default
2. node identity should be cheap to store and compare
3. child relationships should be compact enough for common traversal without excessive indirection
4. the representation should preserve recovery artifacts and diagnostics anchors
5. the representation should be compatible with incremental reuse later

Acceptable implementation families may include:

- arena or index-based trees
- green-tree style immutable compact representations with optional richer views
- other compact node or token-table designs

The exact structure is open, but the invariants above are not.

## Trivia and Source Preservation

The default CST must preserve enough lexical information for source-aware tooling.

That includes:

- comments
- whitespace or trivia when needed for source preservation
- delimiter tokens
- exact spans for all syntactically meaningful tokens

This does not mean every trivia fragment must become a heavy standalone object.

Good implementations may preserve trivia through:

- explicit CST leaf tokens
- compact token buffers referenced by the tree
- source-preserving token streams associated with tree nodes

The key rule is that the framework must preserve source structure without duplicating trivia text everywhere.

## AST Lowering

AST is still a first-class use case, but it should be layered.

Recommended direction:

1. parse to CST by default
2. lower to AST when the caller requests semantic or ergonomic structure
3. allow multiple AST-lowering strategies from the same CST when language needs differ

That means AST should usually be:

- explicit
- opt-in
- lazy or cached when appropriate
- free to discard recovery-only or source-preserving details that the CST retains

## Materialization Strategies

Materialization strategy is the right extension point for higher-level outputs.

Examples:

- CST materialization strategy for the default tree
- typed AST lowering strategy
- event stream materialization strategy
- custom analysis-oriented structure builders

The parser core should not need a separate grammar definition for each of these.

## Event-Capable Core and Tree Building

The internal parse engine should remain event-capable even when CST is the default surface.

That allows:

- CST building as one consumer of parse structure
- AST lowering as a later or alternative consumer
- low-allocation event workflows when a full tree is unnecessary
- a better path to incremental reuse

This is the main way to keep a CST-first architecture competitive.

## Incremental Compatibility

The default tree model should remain incremental-friendly.

That means preserving:

- stable syntax kinds
- stable child ordering rules
- reusable unchanged subtrees or equivalent reusable structure
- source and token identity sufficient for invalidation logic
- profile-aware invalidation when grammar profile changes alter structure assumptions

Incremental parsing may eventually use its own runtime type, but its reuse model should be designed around the CST from the start.

## Likely Types

```rust
pub struct SyntaxTree {
    // compact CST storage
}

pub struct SyntaxNodeId(pub u32);
pub struct SyntaxTokenId(pub u32);

pub struct SyntaxNode {
    pub id: SyntaxNodeId,
    pub kind: SyntaxKind,
    pub span: SourceSpan,
}

pub struct SyntaxToken {
    pub id: SyntaxTokenId,
    pub kind: TokenKind,
    pub span: SourceSpan,
}

pub trait MaterializationStrategy {
    type Output;

    fn consume(&mut self, event: ParseEvent);
    fn finish(self) -> Self::Output;
}

pub trait AstLoweringStrategy {
    type Output;

    fn lower(&self, tree: &SyntaxTree) -> Result<Self::Output, LoweringError>;
}
```

The exact API may differ. The important thing is that `SyntaxTree` is the default stable product, and AST lowering remains layered.

## API Direction

The common path should look like this:

```rust
let tree = parser.parse_tree(&tokens)?;
```

The higher-level path should look like this:

```rust
let tree = parser.parse_tree(&tokens)?;
let ast = AstStrategy::default().lower(&tree)?;
```

If the framework later offers a shorthand such as `parse_ast`, that shorthand should still be implemented as a layered CST-plus-lowering path unless benchmarks prove a different design is necessary.

## Testing Guidance

This area should be tested with both correctness and cost in mind.

Recommended coverage:

1. span and source-preservation correctness
2. recovery artifact retention in CST
3. AST lowering correctness from CST
4. allocation or size regression tracking for common grammars
5. trivia preservation and round-trip-oriented scenarios where relevant
6. incremental reuse compatibility once implemented

## Relationship to Other Specs

- `framework-objectives.md` defines the performance, memory, diagnostics, and DX goals this representation must satisfy
- `parser-execution-and-consumption-models.md` defines how CST, AST, visitors, events, and incremental surfaces relate architecturally
- `source-spans-and-labels.md` defines the span and context information the tree must preserve
- `recovery-and-error-boundaries.md` defines the recovery artifacts the CST must be able to represent or anchor
- `tokenizer-parser-integration-guidelines.md` defines how tokenizer output feeds the parser and tree model coherently

## Open Questions

1. Should trivia be represented as explicit CST tokens, compact sidecar token storage, or a hybrid model?
2. Should the first implementation use a green-tree style representation, an arena-indexed tree, or another compact structure?
3. How much AST lowering should be cached by default versus recomputed on demand?
4. Which stable identity guarantees are required before incremental reuse becomes public API?