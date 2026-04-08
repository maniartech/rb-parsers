use crate::scanners::binary_scanner::BinaryScanner;
use crate::tokens::TokenizationError;

/// A token produced by the [`BinaryTokenizer`].
#[derive(Debug, Clone)]
pub struct BinaryToken {
    /// Token kind.
    pub kind: &'static str,

    /// Optional sub-kind.
    pub sub_kind: Option<&'static str>,

    /// The raw bytes matched by the scanner.
    pub bytes: Vec<u8>,

    /// Byte offset from the start of the input buffer where this token begins.
    pub offset: usize,
}

/// A tokenizer that operates on raw `&[u8]` slices for binary format parsing.
///
/// Register [`BinaryScanner`] implementations with [`add_scanner`](Self::add_scanner),
/// then call [`tokenize_bytes`](Self::tokenize_bytes).  Scanners are tried in
/// registration order; the first match wins.
///
/// # Example — TLV (type-length-value) frame parser
///
/// ```rust,ignore
/// use rb_tokenizer::tokenizers::BinaryTokenizer;
/// use rb_tokenizer::scanners::{BinaryScanner, BinaryScanMatch};
/// use rb_tokenizer::tokens::TokenizationError;
///
/// struct TlvScanner;
///
/// impl BinaryScanner for TlvScanner {
///     fn scan_bytes(&self, input: &[u8]) -> Result<Option<BinaryScanMatch>, TokenizationError> {
///         if input.len() < 2 { return Ok(None); }
///         let len = input[1] as usize;
///         if input.len() < 2 + len { return Ok(None); }
///         Ok(Some(BinaryScanMatch {
///             consumed_bytes: 2 + len,
///             token_kind: "TlvEntry",
///             token_sub_kind: None,
///             bytes: input[..2 + len].to_vec(),
///         }))
///     }
/// }
///
/// let mut tokenizer = BinaryTokenizer::new();
/// tokenizer.add_scanner(Box::new(TlvScanner));
/// let data = &[0x01u8, 0x03, b'a', b'b', b'c'];
/// let tokens = tokenizer.tokenize_bytes(data).unwrap();
/// assert_eq!(tokens.len(), 1);
/// assert_eq!(tokens[0].offset, 0);
/// ```
#[derive(Default)]
pub struct BinaryTokenizer {
    scanners: Vec<Box<dyn BinaryScanner>>,
    /// When `true`, skip one byte and continue on fatal scan errors
    /// instead of aborting and returning `Err`.
    pub continue_on_error: bool,
}

impl BinaryTokenizer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Builder-style helper to set error recovery behaviour.
    pub fn with_continue_on_error(mut self, v: bool) -> Self {
        self.continue_on_error = v;
        self
    }

    /// Register a [`BinaryScanner`].  Scanners are tried in registration order.
    pub fn add_scanner(&mut self, scanner: Box<dyn BinaryScanner>) {
        self.scanners.push(scanner);
    }

    /// Tokenize a raw byte slice.
    ///
    /// Returns `Ok(Vec<BinaryToken>)` on success.  If `continue_on_error` is
    /// `false` (the default), the first [`TokenizationError`] from a scanner
    /// aborts tokenization and returns `Err(Vec<TokenizationError>)`.
    /// If `continue_on_error` is `true`, errors are collected and the tokenizer
    /// skips one byte before retrying.
    pub fn tokenize_bytes(&self, input: &[u8]) -> Result<Vec<BinaryToken>, Vec<TokenizationError>> {
        let mut tokens = Vec::new();
        let mut errors: Vec<TokenizationError> = Vec::new();
        let mut offset = 0;

        while offset < input.len() {
            let remaining = &input[offset..];
            let mut matched = false;

            for scanner in &self.scanners {
                match scanner.scan_bytes(remaining) {
                    Ok(Some(m)) => {
                        tokens.push(BinaryToken {
                            kind: m.token_kind,
                            sub_kind: m.token_sub_kind,
                            bytes: m.bytes,
                            offset,
                        });
                        offset += m.consumed_bytes;
                        matched = true;
                        break;
                    }
                    Ok(None) => {}
                    Err(e) => {
                        errors.push(e);
                        if !self.continue_on_error {
                            return Err(errors);
                        }
                        offset += 1;
                        matched = true;
                        break;
                    }
                }
            }

            if !matched {
                // No scanner claimed this byte — skip it silently.
                offset += 1;
            }
        }

        if errors.is_empty() || self.continue_on_error {
            Ok(tokens)
        } else {
            Err(errors)
        }
    }
}
