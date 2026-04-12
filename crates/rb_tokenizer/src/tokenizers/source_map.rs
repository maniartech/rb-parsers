use memchr::memchr_iter;

/// Prebuilt newline-position index for O(log n) line/column lookup.
///
/// Build once from the full source string with [`SourceMap::new`], then call
/// [`position_of`](Self::position_of) for any byte offset without scanning bytes
/// again.  This replaces the per-token `advance_pos` character loop and removes
/// line/column state variables from the tokenizer hot path entirely.
pub struct SourceMap {
    /// Sorted byte offsets of every `\n` in the source (0-indexed).
    newlines: Vec<usize>,
}

impl SourceMap {
    /// Build a `SourceMap` from `source` using `memchr` for fast newline detection.
    ///
    /// Construction is O(n) in the byte length of `source`.  All subsequent
    /// [`position_of`](Self::position_of) calls are O(log(newline_count)).
    #[inline]
    pub fn new(source: &[u8]) -> Self {
        SourceMap {
            newlines: memchr_iter(b'\n', source).collect(),
        }
    }

    /// Returns `(line, col)` for `byte_offset`, both **0-indexed**.
    ///
    /// `col` is the **byte** distance from the start of the current line to
    /// `byte_offset`.  For ASCII source (JSON, most expression grammars) this
    /// equals the code-point column.  For multi-byte Unicode, divide by the
    /// caller if a code-point column is needed.
    ///
    /// # Panics
    /// Never — `byte_offset` may exceed the source length; the function just
    /// returns the last line/column in that case.
    #[inline]
    pub fn position_of(&self, byte_offset: usize) -> (usize, usize) {
        // Number of newlines *strictly before* byte_offset = 0-based line index.
        let line = self.newlines.partition_point(|&nl| nl < byte_offset);
        let line_start = if line == 0 {
            0
        } else {
            self.newlines[line - 1] + 1
        };
        let col = byte_offset.saturating_sub(line_start);
        (line, col)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_line() {
        let m = SourceMap::new(b"hello");
        assert_eq!(m.position_of(0), (0, 0));
        assert_eq!(m.position_of(3), (0, 3));
    }

    #[test]
    fn two_lines() {
        // "abc\ndef"
        let m = SourceMap::new(b"abc\ndef");
        assert_eq!(m.position_of(0), (0, 0)); // 'a'
        assert_eq!(m.position_of(3), (0, 3)); // '\n'
        assert_eq!(m.position_of(4), (1, 0)); // 'd'
        assert_eq!(m.position_of(6), (1, 2)); // 'f'
    }

    #[test]
    fn three_lines() {
        let m = SourceMap::new(b"a\nb\nc");
        assert_eq!(m.position_of(2), (1, 0)); // 'b'
        assert_eq!(m.position_of(4), (2, 0)); // 'c'
    }
}
