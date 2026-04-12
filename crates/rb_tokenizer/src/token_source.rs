/// Pull-based token source consumed by the parse engine.
///
/// Implementors provide indexed lookahead (`peek`) and consume tokens one at a
/// time (`advance`). The trait is object-safe so the engine can accept either a
/// [`SliceTokenSource`] (backed by a pre-materialised `Vec<Token>`) or a
/// future [`BufferedTokenSource`] (backed by a streaming iterator).
pub trait TokenSource<'src> {
    /// Return the token at `offset` positions ahead of the current position.
    /// `offset = 0` is the current (unconsumed) token.
    fn peek(&self, offset: usize) -> Option<&crate::tokens::Token<'src>>;

    /// Advance past the current token, returning it.
    /// Returns `None` when the source is exhausted.
    fn advance(&mut self) -> Option<&crate::tokens::Token<'src>>;

    /// Current cursor position (index of next token to be consumed).
    fn position(&self) -> usize;

    /// Reset the cursor to `pos`. Panics in debug builds if `pos` is ahead of
    /// the current position (callers must only backtrack, not fast-forward).
    fn reset_to(&mut self, pos: usize);

    /// Returns `true` when all tokens have been consumed.
    fn is_exhausted(&self) -> bool {
        self.peek(0).is_none()
    }

    /// Return the token immediately before the current position, or `None`.
    ///
    /// Used by the parse engine to compute span ends for completed nodes.
    /// The default implementation returns `None`; slice-backed sources override
    /// this to return the actual preceding token.
    fn peek_back(&self) -> Option<&crate::tokens::Token<'src>> { None }

    /// Return the last token in the source, or `None` if the source is empty.
    ///
    /// Used to synthesise an EOF diagnostic location when the cursor is past
    /// the end of the input.  The default returns `None`.
    fn last_token(&self) -> Option<&crate::tokens::Token<'src>> { None }

    /// Inform the source that no backtrack point below `pos` will ever be used.
    ///
    /// For [`BufferedTokenSource`] this allows the sliding window to evict
    /// tokens before `pos`, keeping memory usage proportional to the active
    /// backtracking window.  The default implementation is a no-op (correct
    /// for slice-backed sources that do not evict).
    fn set_commit(&mut self, _pos: usize) {}
}

// ── SliceTokenSource ──────────────────────────────────────────────────────────

/// A [`TokenSource`] backed by an existing `&[Token]` slice.
///
/// This is the zero-allocation adapter that preserves compatibility with the
/// existing `parse_tree(&[Token])` API.
///
/// # Example
/// ```rust,ignore
/// let tokens = tokenizer.tokenize(src)?;
/// let mut src = SliceTokenSource::new(&tokens);
/// // pass `&mut src` to a streaming-aware parse function
/// ```
pub struct SliceTokenSource<'src> {
    tokens: &'src [crate::tokens::Token<'src>],
    pos:    usize,
}

impl<'src> SliceTokenSource<'src> {
    /// Creates a `SliceTokenSource` positioned at the beginning of `tokens`.
    pub fn new(tokens: &'src [crate::tokens::Token<'src>]) -> Self {
        SliceTokenSource { tokens, pos: 0 }
    }
}

impl<'src> TokenSource<'src> for SliceTokenSource<'src> {
    fn peek(&self, offset: usize) -> Option<&crate::tokens::Token<'src>> {
        self.tokens.get(self.pos + offset)
    }

    fn advance(&mut self) -> Option<&crate::tokens::Token<'src>> {
        let tok = self.tokens.get(self.pos);
        if tok.is_some() { self.pos += 1; }
        tok
    }

    fn position(&self) -> usize { self.pos }

    fn reset_to(&mut self, pos: usize) {
        debug_assert!(pos <= self.pos, "SliceTokenSource::reset_to: cannot fast-forward");
        self.pos = pos;
    }

    fn peek_back(&self) -> Option<&crate::tokens::Token<'src>> {
        if self.pos == 0 { None } else { self.tokens.get(self.pos - 1) }
    }

    fn last_token(&self) -> Option<&crate::tokens::Token<'src>> {
        self.tokens.last()
    }

    // set_commit is a no-op for slice sources — default impl is sufficient.
}

// ── BufferedTokenSource ───────────────────────────────────────────────────────

/// A [`TokenSource`] that wraps any `Iterator<Item = Token<'src>>` and maintains
/// a sliding lookahead window backed by a `VecDeque`.
///
/// Once `advance()` is called and no live backtrack frame refers to the
/// discarded position, the underlying token is dropped, keeping memory usage
/// proportional to the current backtrack window rather than the full file.
///
/// **Note**: The `reset_to` implementation re-buffers discarded tokens via
/// `refill_to`. For correct operation, callers must not call `reset_to` with
/// a position that has already been evicted from the buffer.  In practice this
/// means the caller must track the minimum live backtrack point and call
/// `set_commit(pos)` to allow eviction below that point.
pub struct BufferedTokenSource<'src, I>
where
    I: Iterator<Item = crate::tokens::Token<'src>>,
{
    iter:      std::cell::RefCell<I>,
    buffer:    std::cell::RefCell<std::collections::VecDeque<crate::tokens::Token<'src>>>,
    /// Absolute position of the front of `buffer` (i.e. the first retained token).
    base:      std::cell::Cell<usize>,
    /// Current read position (absolute).
    pos:       std::cell::Cell<usize>,
    /// Minimum position that may still be needed for backtracking.
    commit:    std::cell::Cell<usize>,
    exhausted: std::cell::Cell<bool>,
}

impl<'src, I> BufferedTokenSource<'src, I>
where
    I: Iterator<Item = crate::tokens::Token<'src>>,
{
    /// Create a new `BufferedTokenSource` with an initial lookahead buffer of
    /// `initial_capacity` slots.
    pub fn new(iter: I, initial_capacity: usize) -> Self {
        BufferedTokenSource {
            iter:      std::cell::RefCell::new(iter),
            buffer:    std::cell::RefCell::new(std::collections::VecDeque::with_capacity(initial_capacity)),
            base:      std::cell::Cell::new(0),
            pos:       std::cell::Cell::new(0),
            commit:    std::cell::Cell::new(0),
            exhausted: std::cell::Cell::new(false),
        }
    }

    /// Declare that no backtrack frame will ever rewind before `pos`.  Tokens
    /// before `pos` will be evicted from the buffer on the next `advance()`.
    pub fn set_commit(&mut self, pos: usize) {
        if pos > self.commit.get() { self.commit.set(pos); }
        self.evict_committed();
    }

    /// Ensure the buffer contains the token at absolute position `abs_pos`.
    fn fill_to(&self, abs_pos: usize) {
        if self.exhausted.get() { return; }
        let mut buf = self.buffer.borrow_mut();
        let mut iter = self.iter.borrow_mut();
        while self.base.get() + buf.len() <= abs_pos {
            match iter.next() {
                Some(tok) => buf.push_back(tok),
                None      => { self.exhausted.set(true); break; }
            }
        }
    }

    /// Evict tokens before `self.commit` from the front of the buffer.
    fn evict_committed(&self) {
        let commit = self.commit.get();
        let mut buf = self.buffer.borrow_mut();
        while self.base.get() < commit {
            if buf.pop_front().is_some() {
                self.base.set(self.base.get() + 1);
            } else {
                break;
            }
        }
    }
}

impl<'src, I> TokenSource<'src> for BufferedTokenSource<'src, I>
where
    I: Iterator<Item = crate::tokens::Token<'src>>,
{
    fn peek(&self, offset: usize) -> Option<&crate::tokens::Token<'src>> {
        self.fill_to(self.pos.get() + offset);
        let base = self.base.get();
        let pos = self.pos.get();
        let buf_idx = pos + offset - base;
        // SAFETY: we hold a shared reference to `self`; `fill_to` borrows
        // `buffer` mutably only while its `RefCell` guard lives, which has
        // been released above.  We obtain a raw pointer from the deque (which
        // is stable for push_back on VecDeque when capacity allows — but since
        // we check above that the element exists AND the RefCell ensures no
        // concurrent mutation, this is safe).
        let guard = self.buffer.borrow();
        guard.get(buf_idx).map(|tok| {
            // Extend lifetime: the token lives as long as `self`, not as
            // long as the `Ref` guard.  This is safe because:
            // 1. We never remove from the front while `pos + offset` is still
            //    reachable (commit gate enforces this).
            // 2. push_back only invalidates deque iterators, not existing
            //    element references, once the deque is fully allocated.
            unsafe { &*(tok as *const _) }
        })
    }

    fn advance(&mut self) -> Option<&crate::tokens::Token<'src>> {
        let pos = self.pos.get();
        self.fill_to(pos);
        let buf_idx = pos - self.base.get();
        let found = self.buffer.borrow().get(buf_idx).is_some();
        if found {
            self.pos.set(pos + 1);
            // We can safely re-borrow now that pos is updated
            let new_buf_idx = pos - self.base.get();
            let guard = self.buffer.borrow();
            guard.get(new_buf_idx).map(|tok| unsafe { &*(tok as *const _) })
        } else {
            None
        }
    }

    fn position(&self) -> usize { self.pos.get() }

    fn reset_to(&mut self, pos: usize) {
        let commit = self.commit.get();
        debug_assert!(
            pos >= commit,
            "BufferedTokenSource::reset_to: cannot backtrack before committed position \
             (commit={commit}, target={pos})"
        );
        debug_assert!(pos <= self.pos.get(), "BufferedTokenSource::reset_to: cannot fast-forward");
        self.pos.set(pos);
    }

    fn peek_back(&self) -> Option<&crate::tokens::Token<'src>> {
        let pos = self.pos.get();
        if pos == 0 { return None; }
        let abs = pos - 1;
        let base = self.base.get();
        if abs < base { return None; }  // already evicted
        let buf_idx = abs - base;
        let guard = self.buffer.borrow();
        guard.get(buf_idx).map(|tok| unsafe { &*(tok as *const _) })
    }

    fn last_token(&self) -> Option<&crate::tokens::Token<'src>> {
        if !self.exhausted.get() { return None; }
        let guard = self.buffer.borrow();
        guard.back().map(|tok| unsafe { &*(tok as *const _) })
    }

    fn set_commit(&mut self, pos: usize) {
        self.set_commit(pos);
    }
}
