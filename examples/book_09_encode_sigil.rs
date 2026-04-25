#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(clippy::unwrap_used)]
//! See: `book/tutorial/09-encode-sigil.md` for full example
use tinyklv::prelude::*;            // Klv proc-macro + traits
use tinyklv::dec::binary as decb;   // binary decoders
use tinyklv::enc::binary as encb;   // binary encoders

#[derive(Debug, PartialEq, Clone, Copy)]
/// Operating mode, encoded as a single byte
enum Mode {
    Idle,
    Active,
    Error,
}
impl Mode {
    /// Consuming latebind target: u8 value to `Mode`
    fn from_u8(v: u8) -> Self {
        match v {
            0 => Mode::Idle,
            1 => Mode::Active,
            _ => Mode::Error,
        }
    }
}
impl EncodeValue<Vec<u8>> for Mode {
    /// Encoder takes `&Mode`; field attribute uses `enc = Mode::encode_value` (no sigil)
    fn encode_value(&self) -> Vec<u8> {
        encb::u8(match self {
            Mode::Idle   => 0,
            Mode::Active => 1,
            Mode::Error  => 2,
        })
    }
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"HEARTBEAT",
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
        dec = decb::be_u16,
        enc = *encb::be_u16,
    )]
    temperature_centideg: u16,

    #[klv(
        key = 0x03,
        dec = decb::be_u32,
        enc = *encb::be_u32,
    )]
    uptime_s: u32,

    #[klv(
        key = 0x04,
        dec = decb::u8,
        enc = Mode::encode_value,
        latebind = Mode::from_u8,
    )]
    mode: Mode,
}

fn main() {
    // build the original packet
    let original = Heartbeat {
        sequence:             42,
        temperature_centideg: 2350,
        uptime_s:             3600,
        mode:                 Mode::Active,
    };

    // encode_frame: sentinel + body-length + KLV triples
    let frame = original.encode_frame();

    // the same bytes, laid out by hand; proves the derive and your mental
    // model agree before we stop hand-writing streams from Tutorial 10 on
    let expected_frame = [
        // "HEARTBEAT" sentinel
        0x48, 0x45, 0x41, 0x52, 0x54, 0x42, 0x45, 0x41, 0x54,
        0x10,                               // body length = 16 bytes
        0x01, 0x01, 0x2A,                   // sequence             = 42
        0x02, 0x02, 0x09, 0x2E,             // temperature_centideg = 2350
        0x03, 0x04, 0x00, 0x00, 0x0E, 0x10, // uptime_s             = 3600
        0x04, 0x01, 0x01,                   // mode u8              = 1 -> Mode::Active
    ];

    // confirm the frame the derive emits matches the hand-laid bytes
    assert_eq!(frame.as_slice(), expected_frame.as_slice());

    // decode_frame reverses the process
    let decoded = Heartbeat::decode_frame(
        &mut frame.as_slice(),
    ).unwrap();

    // the round-trip produces an identical struct
    assert_eq!(decoded, original);
}
