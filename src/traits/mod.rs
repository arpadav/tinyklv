//! # Trait Architecture
//!
//! Decode and encode have intentionally different trait counts. Decode handles
//! untrusted binary input (fallible, stream-based), while encode serializes
//! known-good Rust structs (infallible, appends into a caller-owned buffer)
//!
//! | Decode             | Encode             | Level          |
//! |--------------------|--------------------|----------------|
//! | `DecodeValue<S>`   | `EncodeValue`      | Value only     |
//! | `SeekSentinel<S>`  | *(sentinel bytes)* | KLV framing    |
//! | `DecodeFrame<S>`   | `EncodeFrame`      | Full pipeline  |
//! | `DrainFrames`      | *(none)*           | Batching       |
//!
//! **Why decode has more traits**: Decode must seek through a byte stream,
//! handle unknown/malformed keys, and recover from partial parses. Encode
//! starts from a valid Rust struct - seeking and error recovery are unnecessary
//!
//! **Encode output**: The encode path appends into a caller-owned `&mut Vec<u8>`
//! ([`EncodeValue::encode_value`] / [`EncodeFrame::encode_frame`]), so a single
//! buffer can be reused across many records. It currently requires `alloc`
//!
//! **`stream` attribute**: Only parameterizes decode. Encode always writes
//! `Vec<u8>`
//!
//! ## The `size(..)` attribute
//!
//! Four keywords, one fact each: `implicit` writes and reads no length prefix, `exact = N` makes
//! the value `N` bytes (decode rejects an item length other than `N`), `lengthed` hands the
//! decoder the value's length (its signature takes the runtime `len`), and `reserve = N` adds `N`
//! bytes to the encoder's up-front allocation, changing no byte on the wire. `exact` and `reserve`
//! are mutually exclusive. Under a fixed-width `len(size(exact = W))` slot, an `exact = N`
//! value's length is written before it and any other value's is back-filled after it; a
//! variable-width `len(..)` stages every value it prefixes. See the book's "Field attributes"
//! reference for the full table.
// --------------------------------------------------
// mods
// --------------------------------------------------
mod coerce;
mod dec;
mod enc;
#[cfg(feature = "bench")]
mod native;

// --------------------------------------------------
// re-exports
// --------------------------------------------------
pub use coerce::EncodeAs;
pub use dec::{
    BreakType, DecodeFrame, DecodePartial, DecodeValue, DrainFrames, Partial, PartialError,
    PartialIterator, ResumePartial, SeekSentinel,
};
pub use enc::{EncodeFrame, EncodeValue};
