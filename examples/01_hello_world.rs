#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(clippy::unwrap_used)]
//! Example 01 - Hello, KLV
//!
//! The smallest useful derive: a two-field heartbeat packet that round-trips
//! through the binary encoder and decoder. Read this first to see how the
//! container attribute sets the key/length codecs once for the whole struct,
//! and how each field attribute converts a single key to its own value codec
//!
//! Showcases:
//! * `#[derive(Klv)]` with `stream`, `sentinel`, `key(...)`, `len(...)`
//! * `EncodeFrame::encode_frame` emitting sentinel + length + KLV triples
//! * `DecodeFrame::decode_frame` seeking the sentinel and rebuilding the struct
//!
//! See also: book Tutorial 01.
use tinyklv::prelude::*;            // Klv proc-macro + traits
use tinyklv::dec::binary as decb;   // binary decoders
use tinyklv::enc::binary as encb;   // binary encoders

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"HEARTBEAT",
    key(dec = decb::u8,          enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
/// Minimal sensor heartbeat: a sequence counter and a temperature reading
struct Heartbeat {
    #[klv(
        key = 0x01,
        dec = decb::u8,
        enc = *encb::u8,
    )]
    /// Monotonic frame counter
    sequence: u8,

    #[klv(
        key = 0x02,
        dec = decb::be_u16,
        enc = *encb::be_u16,
    )]
    /// Temperature in 0.01 C units (big-endian u16)
    temperature_centideg: u16,
}

fn main() {
    // build
    let original = Heartbeat {
        sequence:             42,
        temperature_centideg: 2350, // 23.50 C
    };

    // encode - prepends the sentinel + 1-byte length, then KLV triples
    let mut frame = Vec::new();
    original.encode_frame(&mut frame);

    // decode - seeks the sentinel, reads the length, decodes the value region
    let decoded = Heartbeat::decode_frame(
        &mut frame.as_slice(),
    ).unwrap();

    // assert - full round-trip identity
    assert_eq!(decoded, original);
}
