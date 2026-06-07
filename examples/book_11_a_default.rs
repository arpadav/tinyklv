#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(clippy::unwrap_used)]
//! Book tutorial 11a - `Option<T>` fields and `default = expr` fallbacks
//! See `book/tutorial/11-default-fallback.md` for the full narrative
//!
//! Shows two complementary mechanisms for handling absent keys on decode:
//!
//! * **`Option<T>`**: the field is `None` when the key is missing; encoding
//!   a `None` emits no KLV triple, making the frame shorter
//! * **`default = expr`**: the field takes a compile-time expression when the
//!   key is absent; the field type is plain `T` (not `Option`)
use tinyklv::prelude::*;            // Klv proc-macro + traits
use tinyklv::dec::binary as decb;   // binary decoders
use tinyklv::enc::binary as encb;   // binary encoders

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8,          enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
/// Heartbeat illustrating `Option<T>` absence and `default = expr` fallback
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
        dec = decb::i8,
        enc = *encb::i8,
    )]
    /// Absent in stream = `None`. Encoding a `None` emits no KLV triple
    signal_dbm: Option<i8>,

    #[klv(
        key = 0x03,
        dec = decb::u8,
        enc = *encb::u8,
        default = 100_u8,
    )]
    /// Absent in stream = `100` (the default expression); always emitted on encode
    battery_pct: u8,
}

fn main() {
    // manually constructed stream: only key 0x01 present
    let thin_bytes: &[u8] = &[
        0x01, 0x01, 42,
    ];
    let decoded = Heartbeat::decode_value(
        &mut &thin_bytes[..],
    ).unwrap();

    assert_eq!(decoded.sequence, 42);
    // signal_dbm was never in stream -> Option stayed None
    assert_eq!(decoded.signal_dbm, None);
    // battery_pct was never in stream -> default expression 100 survived
    assert_eq!(decoded.battery_pct, 100);

    // round-trip a full packet
    let full = Heartbeat {
        sequence:    7,
        signal_dbm:  Some(-54),
        battery_pct: 87,
    };
    let mut full_bytes = Vec::new();
    full.encode_value(&mut full_bytes);
    let full_decoded = Heartbeat::decode_value(
        &mut full_bytes.as_slice(),
    ).unwrap();
    assert_eq!(full_decoded, full);

    // when signal_dbm is None the encoded bytes are strictly shorter -
    // a `None` field emits no KLV triple
    let thin = Heartbeat { signal_dbm: None, ..full };
    let mut thin_bytes = Vec::new();
    thin.encode_value(&mut thin_bytes);
    assert!(thin_bytes.len() < full_bytes.len());
}
