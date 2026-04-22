#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(clippy::unwrap_used)]
//! See: `book/tutorial/10-default-fallback.md` for full example
use tinyklv::prelude::*;            // Klv proc-macro + traits
use tinyklv::dec::binary as decb;   // binary decoders
use tinyklv::enc::binary as encb;   // binary encoders

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8,          enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct Heartbeat {
    #[klv(
        key = 0x01,
        dec = decb::u8,
        enc = *encb::u8,
    )]
    sequence: u8,

    #[klv(
        key = 0x02,
        dec = decb::i8,
        enc = *encb::i8,
    )]
    /// Absent on the wire = `None`. Encoding a `None` emits no KLV triple
    signal_dbm: Option<i8>,

    #[klv(
        key = 0x03,
        dec = decb::u8,
        enc = *encb::u8,
        default = 100_u8,
    )]
    /// Absent on the wire = `100` (the default expression); always emitted on encode
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
    // signal_dbm was never on the wire -> Option stayed None
    assert_eq!(decoded.signal_dbm, None);
    // battery_pct was never on the wire -> default expression 100 survived
    assert_eq!(decoded.battery_pct, 100);

    // round-trip a full packet
    let full = Heartbeat {
        sequence:    7,
        signal_dbm:  Some(-54),
        battery_pct: 87,
    };
    let full_bytes = full.encode_value();
    let full_decoded = Heartbeat::decode_value(
        &mut full_bytes.as_slice(),
    ).unwrap();
    assert_eq!(full_decoded, full);

    // when signal_dbm is None the encoded bytes are strictly shorter -
    // a `None` field emits no KLV triple
    let thin = Heartbeat { signal_dbm: None, ..full };
    assert!(thin.encode_value().len() < full_bytes.len());
}
