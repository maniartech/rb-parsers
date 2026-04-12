use rb_common::diagnostics::Diagnostic;
use rb_common::spans::{SourcePosition, SourceSpan};
use crate::cst::SyntaxKind;
use crate::engine::RecoveryAction;

// PERF: `Diagnostic` is a large heap-owning struct (String + several Vecs).
// Boxing it here keeps `ParseEvent` at ~112 bytes (Token variant dominates)
// instead of ~200+ bytes, which is critical for `EventCollectingStrategy`
// throughput (fewer cache misses, cheaper Vec push / growth).

/// Push-model event stream emitted by the parse engine.
///
/// Both `CstBuildingStrategy` and `EventCollectingStrategy` consume this
/// stream. Custom strategies may also consume it directly.
#[derive(Debug, Clone)]
pub enum ParseEvent {
    /// A syntax node boundary has opened.
    NodeStart {
        /// The syntactic category of the opening node.
        kind: SyntaxKind,
        /// Source position at which this node begins.
        span_start: SourcePosition,
    },
    /// The previously opened node has closed.
    NodeEnd {
        /// The syntactic category of the closed node.
        kind: SyntaxKind,
        /// Full source span of the closed node.
        span: SourceSpan,
    },
    /// A leaf token was consumed.
    Token {
        /// Scanner-assigned token type name.
        token_type: &'static str,
        /// Optional sub-kind (from `tok_sub` combinators).
        token_sub_kind: Option<&'static str>,
        /// Source span of this token.
        span: SourceSpan,
        /// `true` for whitespace, comments, and other trivia.
        is_trivia: bool,
        /// Field label if this token was inside a `field(…)` combinator.
        field_name: Option<&'static str>,
    },
    /// Named field boundary was entered.
    FieldStart {
        /// The field label, as declared in the grammar.
        name: &'static str,
    },
    /// Named field boundary was exited.
    FieldEnd {
        /// The field label, matching the preceding `FieldStart`.
        name: &'static str,
    },
    /// A diagnostic was emitted at this point in the stream.
    Error {
        /// The boxed diagnostic value.
        diagnostic: Box<Diagnostic>,
    },
    /// The engine applied a recovery action.
    Recovery {
        /// The recovery action that was taken.
        action: RecoveryAction,
    },
}
