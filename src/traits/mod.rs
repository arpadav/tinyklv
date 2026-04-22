//! # Trait Architecture
//!
//! Decode and encode have intentionally different trait counts. Decode handles
//! untrusted binary input (fallible, stream-based), while encode serializes
//! known-good Rust structs (infallible, returns owned bytes).
//!
//! | Decode             | Encode             | Level          |
//! |--------------------|--------------------|----------------|
//! | `DecodeValue<S>`   | `EncodeValue<O>`   | Value only     |
//! | `SeekSentinel<S>`  | *(sentinel bytes)* | KLV framing    |
//! | `DecodeFrame<S>`   | `EncodeFrame<O>`   | Full pipeline  |
//! | `BreakCondition`   | *(none)*           | Loop control   |
//! | `DrainFrames`   | *(none)*           | Batching       |
//!
//! **Why decode has more traits**: Decode must seek through a byte stream,
//! handle unknown/malformed keys, and recover from partial parses. Encode
//! starts from a valid Rust struct - seeking and error recovery are unnecessary.
//!
//! **Encode output**: The encode path currently requires `alloc` (`Vec<u8>`).
//! The [`EncodedOutput`] trait is the escape hatch for hand-written non-`Vec<u8>`
//! implementations. A future `encode_into(&self, buf: &mut [u8])` path is
//! desirable for embedded targets.
//!
//! **`stream` attribute**: Only parameterizes decode. Encode always produces
//! `Vec<u8>`. The `EncodedOutput` trait exists for non-`Vec<u8>` targets via
//! hand-written impls.
//!
//! **`varlen` attribute**: Decode-only. Controls whether `(len)` is passed to the
//! decoder function. Encoding does not use it.
// --------------------------------------------------
// mods
// --------------------------------------------------
mod coerce;
mod dec;
mod enc;
mod types;

// --------------------------------------------------
// local
// --------------------------------------------------
pub use coerce::*;
pub use dec::*;
pub use enc::*;
pub use types::*;
