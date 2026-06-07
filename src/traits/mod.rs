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
//! Every value has an encoder with two paths (*fixed* - always `N` bytes; *dynamic* - size varies,
//! so the length is back-patched after the body) and a decoder with two paths (*internal* - knows
//! its width or self-delimits on the `len`-bounded sub-slice; *uses-len* - the signature takes the
//! runtime length). `size(..)` controls both, along two orthogonal axes: `var` (the decoder takes
//! the runtime `len`; absent ⇒ internal) and the byte count `exact = N` (wire-exact ⇒ fixed-width
//! fast path) or `hint = N` (a soft encode-capacity estimate only). `exact` and `hint` are mutually
//! exclusive. See the book's "encoding model" reference for the full strategy/path table.
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
pub use coerce::*;
pub use dec::*;
pub use enc::*;
