use crate::tokens::TokenizationError;

/// Result returned by a [`BinaryScanner`] on a successful match.
pub struct BinaryScanMatch {
    /// Number of bytes consumed from the beginning of the input slice.
    pub consumed_bytes: usize,

    /// The token kind identifier.
    pub token_kind: &'static str,

    /// Optional token sub-kind.
    pub token_sub_kind: Option<&'static str>,

    /// The matched bytes (cloned from the input slice for ownership).
    pub bytes: Vec<u8>,
}

/// A scanner that operates directly on raw byte slices instead of UTF-8 strings.
///
/// Use this to tokenize binary formats — network protocol frames, binary file
/// headers, custom wire encodings, etc.  Register implementations with
/// [`BinaryTokenizer`](crate::tokenizers::BinaryTokenizer).
///
/// # Example — fixed-width big-endian u32 field
///
/// ```rust,ignore
/// use rb_tokenizer::scanners::{BinaryScanner, BinaryScanMatch};
/// use rb_tokenizer::tokens::TokenizationError;
///
/// pub struct U32Scanner;
///
/// impl BinaryScanner for U32Scanner {
///     fn scan_bytes(&self, input: &[u8]) -> Result<Option<BinaryScanMatch>, TokenizationError> {
///         if input.len() < 4 {
///             return Ok(None);
///         }
///         Ok(Some(BinaryScanMatch {
///             consumed_bytes: 4,
///             token_kind: "U32",
///             token_sub_kind: None,
///             bytes: input[..4].to_vec(),
///         }))
///     }
/// }
/// ```
pub trait BinaryScanner: Send + Sync {
    /// Attempt to scan a token from the beginning of `input`.
    ///
    /// * `Ok(Some(m))` — matched `m.consumed_bytes` bytes.
    /// * `Ok(None)`    — this scanner does not match; try the next one.
    /// * `Err(e)`      — fatal scan error (e.g. truncated fixed-length field).
    fn scan_bytes(&self, input: &[u8]) -> Result<Option<BinaryScanMatch>, TokenizationError>;
}
