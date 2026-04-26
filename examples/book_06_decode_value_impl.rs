#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(clippy::unwrap_used)]
//! See: `book/tutorial/06-decode-value-impl.md` for full example
use tinyklv::prelude::*;            // Klv proc-macro, traits
use tinyklv::dec::binary as decb;   // binary decoders

#[derive(Debug, PartialEq, Clone, Copy)]
/// Degrees Celsius, encoded as big-endian `u16` centidegrees
struct Celsius(f32);

/// `Celsius` carries its own contract; no free-standing decoder fn required
impl DecodeValue<&[u8]> for Celsius {
    fn decode_value(input: &mut &[u8]) -> tinyklv::Result<Self> {
        let centideg = decb::be_u16(input)?;
        Ok(Celsius(centideg as f32 / 100.0))
    }
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"HEARTBEAT",
    key(dec = decb::u8),
    len(dec = decb::u8_as_usize),
    default(typ = u8,  dec = decb::u8),
    default(typ = u32, dec = decb::be_u32),
    default(typ = Celsius, dec = Celsius::decode_value),
    allow_unimplemented_encode,
)]
struct Heartbeat {
    #[klv(key = 0x01)]  sequence:    u8,
    #[klv(key = 0x02)]  temperature: Celsius,
    #[klv(key = 0x03)]  battery_pct: u8,
    #[klv(key = 0x04)]  rssi_dbm:    u8,
    #[klv(key = 0x05)]  uptime_s:    u32,
    #[klv(key = 0x06)]  mode_flags:  u8,
}

fn main() {
    // manually construct the stream
    let stream = [
        0xDE, 0xAD, 0xBE, 0x00,             // junk preamble, no sentinel here
        // "HEARTBEAT" sentinel
        0x48, 0x45, 0x41, 0x52, 0x54, 0x42, 0x45, 0x41, 0x54,
        0x16,                               // body length = 22 bytes
        0x01, 0x01, 0x2A,                   // sequence         = 42
        0x02, 0x02, 0x09, 0x2E,             // temperature      = 2350 centideg -> 23.50 C
        0x03, 0x01, 0x57,                   // battery_pct      = 87
        0x04, 0x01, 0xB8,                   // rssi_dbm         = 0xB8
        0x05, 0x04, 0x00, 0x00, 0x0E, 0x10, // uptime_s         = 3600
        0x06, 0x01, 0x03,                   // mode_flags       = 0b0000_0011
    ];

    // manually construct the expected value
    let expected = Heartbeat {
        sequence:    42,
        temperature: Celsius(23.50),
        battery_pct: 87,
        rssi_dbm:    0xB8,
        uptime_s:    3600,
        mode_flags:  0b0000_0011,
    };

    // seek sentinel, decode the value
    let decoded = Heartbeat::decode_frame(
        &mut stream.as_slice(),
    ).unwrap();

    // they equal!
    assert_eq!(decoded, expected);
}
