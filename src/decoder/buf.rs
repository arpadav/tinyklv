//! Growing intake buffer with an O(1) consumed-prefix cursor for [`crate::decoder::Decoder`]
//!
//! [`BufCursor`] owns the decoder's byte buffer and the single offset (`head`) that separates
//! already-decoded bytes from the live, not-yet-decoded tail. Every slice into the buffer goes
//! through one of two accessors here, so the `head <= buf.len()` invariant - and therefore the
//! in-bounds-ness of all decoder indexing - is proven in exactly one place instead of being
//! re-argued at each call site
//!
//! Author: aav

/// Growing intake buffer paired with a consumed-prefix cursor
///
/// The live (not-yet-decoded) bytes are `buf[head..]`; the prefix `buf[..head]` is
/// consumed-but-not-yet-reclaimed and is dropped in bulk by [`BufCursor::extend`]. Advancing the
/// cursor past a decoded packet is O(1) (`head += n`); the buffer is only compacted on feed, and
/// even then only when reclaiming frees at least as many bytes as it shifts
///
/// # Invariant
///
/// `head <= buf.len()` at all times. The fields are private and every method preserves this, so
/// no external edit can desynchronise the cursor from the buffer
#[derive(Debug)]
pub(super) struct BufCursor {
    /// Backing storage; grows on [`Self::extend`] and is compacted from the front there
    buf: Vec<u8>,

    /// Offset of the first undecoded byte; `0 <= head <= buf.len()`
    head: usize,
}
/// [`BufCursor`] implementation
impl BufCursor {
    /// Constructs an empty cursor with no buffered bytes
    ///
    /// # Returns
    ///
    /// A [`BufCursor`] whose buffer is empty and whose `head` is `0`
    #[inline]
    pub(super) fn new() -> Self {
        Self {
            buf: Vec::new(),
            head: 0,
        }
    }

    /// Constructs an empty cursor with a pre-allocated backing buffer
    ///
    /// # Arguments
    ///
    /// * `cap` - Initial byte capacity passed to [`Vec::with_capacity`]
    ///
    /// # Returns
    ///
    /// A [`BufCursor`] with capacity `cap`, no buffered bytes, and `head` of `0`
    #[inline]
    pub(super) fn with_capacity(cap: usize) -> Self {
        Self {
            buf: Vec::with_capacity(cap),
            head: 0,
        }
    }

    /// Borrows the live, not-yet-decoded bytes (`buf[head..]`)
    ///
    /// # Returns
    ///
    /// A `&[u8]` view of the bytes from `head` to the end; empty when all data has been decoded
    #[inline]
    #[allow(
        clippy::indexing_slicing,
        reason = "head <= buf.len() is a struct invariant upheld by every BufCursor method (head only \
                  advances by amounts <= pending().len(), and is reset to 0 on extend/clear), so buf[head..] \
                  is always in-bounds"
    )]
    pub(super) fn pending(&self) -> &[u8] {
        &self.buf[self.head..]
    }

    /// Borrows the `n` bytes consumed by a single decode step, anchored at the pre-advance `start`
    ///
    /// Used to build winnow error context that reports exactly the bytes a failing step touched
    ///
    /// # Arguments
    ///
    /// * `start` - The value of [`Self::head`] captured before the step advanced the cursor
    /// * `n` - The number of bytes the step consumed (`before - cursor.len()`)
    ///
    /// # Returns
    ///
    /// A `&[u8]` view of `buf[start..start + n]`
    #[inline]
    #[allow(
        clippy::indexing_slicing,
        reason = "callers pass start = a prior head value (<= buf.len()) and n = before - cursor.len(); \
                  the cursor is a suffix of buf[start..] and winnow's Stream only ever advances it, so \
                  n <= before = buf.len() - start, giving start + n <= buf.len()"
    )]
    pub(super) fn consumed_slice(&self, start: usize, n: usize) -> &[u8] {
        debug_assert!(
            start <= self.buf.len() && n <= self.buf.len() - start,
            "consumed_slice out of bounds would break the head <= buf.len() proof"
        );
        &self.buf[start..start + n]
    }

    /// Returns the current cursor offset (`head`)
    ///
    /// # Returns
    ///
    /// The index of the first undecoded byte; callers pin this as `start` before a decode step
    #[inline]
    pub(super) fn head(&self) -> usize {
        self.head
    }

    /// Reports whether every buffered byte has been consumed
    ///
    /// # Returns
    ///
    /// `true` when `head == buf.len()` (no live bytes remain), `false` otherwise
    #[inline]
    pub(super) fn is_exhausted(&self) -> bool {
        self.head == self.buf.len()
    }

    /// Advances the cursor past `n` consumed bytes in O(1)
    ///
    /// No bytes are moved; the consumed prefix is reclaimed later by [`Self::extend`]
    ///
    /// # Arguments
    ///
    /// * `n` - The number of bytes just consumed; must not exceed [`Self::pending`]'s length
    #[inline]
    pub(super) fn advance(&mut self, n: usize) {
        debug_assert!(
            n <= self.pending().len(),
            "advance past end of live bytes would break head <= buf.len()"
        );
        self.head += n;
    }

    /// Reclaims the consumed prefix when worthwhile, then appends new bytes
    ///
    /// Compaction is deferred and conditional so a burst of [`Self::advance`] calls between two
    /// feeds moves no bytes. When the buffer is fully consumed the prefix is dropped in O(1) via
    /// [`Vec::clear`] (no memmove). Otherwise the front is drained only when the freed prefix is at
    /// least as large as the live tail it must shift (`2 * head >= len`), which bounds total
    /// shifting to O(total bytes consumed) and avoids the "small prefix, large growing tail" O(n^2)
    /// trap of compacting on every feed
    ///
    /// # Arguments
    ///
    /// * `bytes` - The new bytes to append after any reclamation
    pub(super) fn extend(&mut self, bytes: &[u8]) {
        // --------------------------------------------------
        // reclaim the consumed prefix, but only when cheap
        // --------------------------------------------------
        if self.head == self.buf.len() {
            self.buf.clear(); // fully consumed: drop everything in O(1), no memmove
            self.head = 0;
        } else if self.head >= self.buf.len() - self.head {
            // --------------------------------------------------
            // freed prefix >= shifted tail (overflow-proof form of `2*head >= len`):
            // bounds total shifting to O(total bytes consumed) over the stream
            // --------------------------------------------------
            self.buf.drain(..self.head);
            self.head = 0;
        }
        // --------------------------------------------------
        // append
        // --------------------------------------------------
        self.buf.extend_from_slice(bytes);
    }

    /// Drops all bytes and resets the cursor to the empty state
    #[inline]
    pub(super) fn clear(&mut self) {
        self.buf.clear();
        self.head = 0;
    }
}
