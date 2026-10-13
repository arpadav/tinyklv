#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(clippy::unwrap_used)]
//! Book tutorial 19 - `unknown(dec, enc)` and the `#[klv(unknown)]` field
//! See `book/tutorial/19-unknown.md` for the full narrative
//!
//! A heartbeat that declares two of its keys. `dec_frame` reads the frames of
//! every other key, the `extra` field keeps them as `(key, value)` pairs, and
//! `enc_frame` writes them back after the declared fields
use tinyklv::prelude::*;            // Klv proc-macro + traits
use tinyklv::dec::binary as decb;   // binary decoders
use tinyklv::enc::binary as encb;   // binary encoders

/// Reads the rest of a frame whose key no field declares: its length, then that
/// many bytes. A frame cut short consumes the input to its end, which the
/// decode loop reads as truncation
fn dec_frame(_key: u8) -> impl Fn(&mut &[u8]) -> tinyklv::Result<Vec<u8>> {
    |input| {
        let len = decb::u8_as_usize(input)?;
        let Some((value, rest)) = input.split_at_checked(len) else {
            input.finish();
            return Err(winnow::error::ContextError::from_input(input));
        };
        *input = rest;
        Ok(value.to_vec())
    }
}

/// Writes the rest of a kept frame, after its key: its length, then its bytes
fn enc_frame(value: &[u8], out: &mut Vec<u8>) {
    encb::u8_from_usize(value.len(), out);
    out.extend_from_slice(value);
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(dec = decb::u8,          enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
    default(typ = u8, dec = decb::u8, enc = *encb::u8),
    unknown(dec = dec_frame, enc = &enc_frame),
)]
/// Heartbeat that keeps the frames of the keys it does not declare
struct Heartbeat {
    #[klv(key = 0x01)]
    /// Monotonic frame counter
    sequence: u8,

    #[klv(key = 0x03)]
    /// Battery charge 0..=100 %
    battery_pct: u8,

    #[klv(unknown)]
    /// Every frame whose key no field declares, in wire order
    extra: Vec<(u8, Vec<u8>)>,
}

fn main() {
    // manually construct the stream
    let stream = [
        // sequence = 42
        0x01, 0x01, 0x2A,
        // key 0x02, no field: kept
        0x02, 0x02, 0x09, 0x2E,
        // battery_pct = 87
        0x03, 0x01, 0x57,
        // key 0x04, no field: kept
        0x04, 0x01, 0xB8,
    ];

    // manually construct the expected value
    let expected = Heartbeat {
        sequence:    42,
        battery_pct: 87,
        extra:       vec![(0x02, vec![0x09, 0x2E]), (0x04, vec![0xB8])],
    };

    // decode: the unknown frames land in `extra`
    let decoded = Heartbeat::decode_value(
        &mut stream.as_slice(),
    ).unwrap();
    assert_eq!(decoded, expected);

    // encode: the declared fields, then the kept frames in the order decoded
    let reencoded = [
        // sequence = 42
        0x01, 0x01, 0x2A,
        // battery_pct = 87
        0x03, 0x01, 0x57,
        // key 0x02, kept
        0x02, 0x02, 0x09, 0x2E,
        // key 0x04, kept
        0x04, 0x01, 0xB8,
    ];
    let mut encoded = Vec::new();
    decoded.encode_value(&mut encoded);
    assert_eq!(encoded, reencoded);
}
