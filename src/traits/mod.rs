//! # Trait Architecture
//!
//! Decode and encode have intentionally different trait counts. Decode handles
//! untrusted binary input (fallible, stream-based), while encode serializes
//! known-good Rust structs (infallible, returns owned bytes).
//!
//! | Decode             | Encode             | Level          |
//! |--------------------|--------------------|----------------|
//! | `DecodeValue<S>`   | `EncodeValue`      | Value only     |
//! | `SeekSentinel<S>`  | *(sentinel bytes)* | KLV framing    |
//! | `DecodeFrame<S>`   | `EncodeFrame`      | Full pipeline  |
//! | *(`break_on` attr -> [`BreakType`])* | *(none)* | Loop control   |
//! | `DrainFrames`   | *(none)*           | Batching       |
//!
//! **Why decode has more traits**: Decode must seek through a byte stream,
//! handle unknown/malformed keys, and recover from partial parses. Encode
//! starts from a valid Rust struct - seeking and error recovery are unnecessary.
//!
//! **Encode output**: The encode path appends into a caller-owned `&mut Vec<u8>`
//! ([`EncodeValue::encode_value`] / [`EncodeFrame::encode_frame`]), so a single
//! buffer can be reused across many records. It currently requires `alloc`.
//!
//! **`stream` attribute**: Only parameterizes decode. Encode always writes
//! `Vec<u8>`.
//!
//! **`varlen` attribute**: Decode-only. Controls whether `(len)` is passed to the
//! decoder function. Encoding does not use it.
// --------------------------------------------------
// mods
// --------------------------------------------------
mod coerce;
mod dec;
mod enc;
#[cfg(feature = "bench")]
mod native;

// --------------------------------------------------
// local
// --------------------------------------------------
pub use coerce::*;
pub use dec::*;
pub use enc::*;
