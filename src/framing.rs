//! The [`Framing`] returned by a fused `#[klv(keylen(dec = ..))]` decoder to describe a value frame
//!
//! Classic `#[klv(key = .., len = ..)]` always slices each value via `take(len)`. The opt-in fused
//! `keylen` codec instead decodes one frame header and returns `(key, Framing)`, letting the framing
//! decision depend on the key. [`Framing`] tells the derive-generated decode loop how to bound the
//! value bytes that follow the header:
//!
//! * [`Framing::Sliced`] carries an explicit byte length - the loop pre-checks the remaining input,
//!   `take`s that many bytes, and hands the value decoder a bounded sub-slice (the classic shape).
//! * [`Framing::Inline`] carries no length - the value is self-delimiting (e.g. a varint), so the
//!   loop emits no `take` and no pre-check and lets the value decoder read straight from the main
//!   stream. A well-behaved value decoder errors on incomplete input; the loop then resets to the
//!   per-frame checkpoint, so streaming resume stays correct.
//!
//! Author: aav

/// How a fused `keylen` decoder frames the value bytes following a key
///
/// Returned as the second element of a `keylen` decoder's `(key, Framing)` tuple. The
/// derive-generated decode loop branches on this value to decide whether to slice the value into a
/// bounded sub-slice or let the value decoder consume it inline from the main stream
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Framing {
    /// The value occupies exactly `usize` bytes, bounded by an explicit length
    ///
    /// The decode loop pre-checks that at least this many bytes remain, `take`s them, and runs the
    /// value decoder against the resulting sub-slice. This is the framing every classic
    /// `key`+`len` field uses, and the one wire types that carry a length prefix (or a fixed width)
    /// resolve to
    Sliced(usize),

    /// The value is self-delimiting and carries no length on the wire
    ///
    /// The decode loop emits neither a `take` nor an `eof < len` pre-check; the value decoder reads
    /// directly from the main stream and stops itself (e.g. a varint terminates on its continuation
    /// bit). On a value-decode error the loop resets to the per-frame checkpoint, yielding a clean
    /// streaming resume. This is the path that eliminates the double scan of a self-delimiting value
    Inline,
}
