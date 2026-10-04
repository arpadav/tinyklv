//! The [`Framing`] returned by a fused `#[klv(keylen(dec = ..))]` decoder to describe a value frame
//!
//! Classic `#[klv(key(..), len(..))]` always slices each value via `take(len)`. The opt-in fused
//! `keylen` codec instead decodes one frame header and returns `(key, Framing)`, letting the framing
//! decision depend on the key. [`Framing`] tells the derive-generated decode loop how to bound the
//! value bytes that follow the header, and states the rules the loop applies to each frame
//!
//! A `keylen` container generates decode impls only, and does not support `sentinel` or
//! `break_on`. Here key `0x01` is length-prefixed and key `0x02` is an `Inline` BER-OID varint:
//!
//! ```rust
//! use tinyklv::{dec::ber as decber, dec::binary as decb, prelude::*};
//!
//! fn header(input: &mut &[u8]) -> tinyklv::Result<(u8, Framing)> {
//!     let key = decb::u8(input)?;
//!     let framing = match key {
//!         0x02 => Framing::Inline,
//!         _ => Framing::Sliced(decb::u8_as_usize(input)?),
//!     };
//!     Ok((key, framing))
//! }
//!
//! #[derive(Klv, Debug, PartialEq)]
//! #[klv(keylen(dec = header), allow_unimplemented_encode)]
//! struct Reading {
//!     #[klv(key = 0x01, dec = decb::u8)]
//!     id: u8,
//!     #[klv(key = 0x02, dec = decber::ber_oid)]
//!     count: u64,
//! }
//!
//! let bytes = [
//!     // key 0x01, length 1, id
//!     0x01, 0x01, 0x07,
//!     // key 0x02, no length, count = 128 as a two-byte varint
//!     0x02, 0x81, 0x00,
//! ];
//! assert_eq!(
//!     Reading::decode_value(&mut &bytes[..]),
//!     Ok(Reading { id: 7, count: 128 }),
//! );
//! ```
//!
//! Author: aav

/// How a fused `keylen` decoder frames the value bytes following a key
///
/// Returned as the second element of a `keylen` decoder's `(key, Framing)` tuple. The
/// derive-generated decode loop branches on this value to decide whether to slice the value into a
/// bounded sub-slice or let the value decoder consume it inline from the main stream
///
/// The key must determine the framing. The loop picks a field by its key alone and runs that
/// field's decoder on whatever framing the header returned, so an `Inline` frame for a known key
/// feeds that field's decoder the unbounded main stream. A decoder written for a bounded sub-slice
/// (one that reads to the end of its input, such as a string) then reads past its own value. Have
/// the `keylen` decoder return one framing per key, the one the key's field decoder expects
///
/// Any failure of the `keylen` decoder itself is truncation, wherever the cursor stopped: the loop
/// rewinds to the start of the frame, then streaming decode returns `Packet::NeedMore` and one-shot
/// decode finalizes what it has. A header codec such as `be_u16` fails short of the end of the
/// input when a chunk boundary splits it, so the end-of-input test the `Inline` values use would
/// class a split header as malformed and drop the rest of the message.
/// The cost: a garbage header in a complete buffer is never reported as malformed. Streaming decode
/// waits for more bytes, and one-shot decode finalizes without the frame
///
/// Without `deny_unknown_keys`, a key with no field is skipped when [`Framing::Sliced`]. When
/// [`Framing::Inline`] it has no length to skip it by, so the loop rewinds to the start of its
/// frame and ends. With `deny_unknown_keys`, an unknown key is rejected in either framing, before
/// any of its value bytes are read
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
    /// bit). This is the path that eliminates the double scan of a self-delimiting value
    ///
    /// The value decoder must return [`crate::Result<T>`], whose error is winnow's
    /// `ContextError`. A decoder with another error type is a compile error, reported at the
    /// decoder
    ///
    /// When the value decoder fails, the cursor position decides what happened:
    ///
    /// * at the end of the input, the value is truncated: the loop resets to the start of the
    ///   frame, then streaming decode returns `Packet::NeedMore` and one-shot decode finalizes
    ///   what it has
    /// * anywhere else, the value is malformed. Streaming decode returns an error naming
    ///   `` `<Struct>::<field>` ``. One-shot decode returns the decoder's own error with that name
    ///   added as context, which the error's display shows first only when the decoder's error has
    ///   no label of its own
    ///
    /// A decoder that rejects short input without consuming it fails away from the end, so its
    /// truncation reads as malformed. Frame such values as [`Framing::Sliced`] instead, whose
    /// length pre-check catches truncation before the decoder runs
    ///
    /// The converse also holds: a decoder that consumes a complete value and then rejects it
    /// without rewinding fails at the end of the input when that value is the last one, so its
    /// malformed value reads as truncation. Rewind to the start of the value before rejecting it
    Inline,
}
