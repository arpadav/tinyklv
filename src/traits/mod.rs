//! # Trait Architecture
//!
//! Decode and encode have intentionally different trait counts. Decode handles
//! untrusted binary input (fallible, stream-based), while encode serializes
//! known-good Rust structs (infallible, returns owned bytes).
//!
//! | Decode            | Encode             | Level          |
//! |-------------------|--------------------|----------------|
//! | `Decode<S>`       | `EncodeValue<O>`   | Value only     |
//! | `Seek<S>`         | *(sentinel bytes)* | KLV framing    |
//! | `Extract<S>`      | `Encode<O>`        | Full pipeline  |
//! | `BreakCondition`  | *(none)*           | Loop control   |
//! | `RepeatedDecode`  | *(none)*           | Batching       |
//! | `ThenDecode`      | *(none)*           | Internal       |
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
//! **`var` attribute**: Decode-only. Controls whether `(len)` is passed to the
//! decoder function. Encoding does not use it.

// --------------------------------------------------
// mods
// --------------------------------------------------
mod dec;
mod enc;
mod types;

// --------------------------------------------------
// local
// --------------------------------------------------
pub use dec::*;
pub use enc::*;
pub use types::*;

#[allow(dead_code)]
/// A fixed length decoder function signature
///
/// **This type is for documentation purposes only**
///
/// `fn <name>(input: &mut S) -> tinyklv::Result<Self>`
type FixedDecodeSignature = ();

#[allow(dead_code)]
/// A variable length decoder function signature
///
/// **This type is for documentation purposes only**
///
/// `fn <name>(len: usize) -> impl Fn(&mut S) -> tinyklv::Result<Self>`
type VariableDecodeSignature = ();

/// A type
pub enum Length {
    /// An explicit length of fixed size, in bytes.
    ///
    /// For example, an unsigned 16 bit integer is [`Length::Fixed(2)`](Length::Fixed)
    ///
    /// Decoding function signatures for [`Length::Fixed`] types are required to be
    /// [`FixedDecodeSignature`]
    Fixed(usize),

    /// An implied length determined during decoding.
    ///
    /// This varies from [`Length::Fixed`] and [`Length::Variable`], in the
    /// sense that:
    ///
    /// 1. Same function signature as [`Length::Fixed`], but no explicit length
    ///    checking can be performed.
    /// 2. Different function signature as [`Length::Variable`]
    ///
    /// Decoding function signatures for [`Length::Implicit`] types are required to be
    /// [`FixedDecodeSignature`]
    Implicit,

    /// An explicitly defined variable length.
    ///
    /// For example, a string
    ///
    /// Decoding function signatures for [`Length::Variable`] types are required to be
    /// [`VariableDecodeSignature`]
    Variable,
}
