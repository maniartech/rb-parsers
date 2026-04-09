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
        kind: SyntaxKind,
        span_start: SourcePosition,
    },
    /// The previously opened node has closed.
    NodeEnd {
        kind: SyntaxKind,
        span: SourceSpan,
    },
    /// A leaf token was consumed.
    Token {
        token_type: &'static str,
        token_sub_kind: Option<&'static str>,
        span: SourceSpan,
        is_trivia: bool,
        field_name: Option<&'static str>,
    },
    /// Named field boundary was entered.
    FieldStart { name: &'static str },
    /// Named field boundary was exited.
    FieldEnd { name: &'static str },
    /// A diagnostic was emitted at this point in the stream.
    Error { diagnostic: Box<Diagnostic> },
    /// The engine applied a recovery action.
    Recovery { action: RecoveryAction },
}
