#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(clippy::unwrap_used)]
//! Book tutorial 04 - container-level `default(typ = ...)` codecs
//! See `book/tutorial/04-default-codec.md` for the full narrative
//!
//! Demonstrates how to register type-wide decoders at the container level so
//! that fields of those types only need a `key = ...` attribute. The
//! `Heartbeat` here has six fields of three different primitive types; all six
//! decoders are resolved from the container defaults rather than repeated on
//! each field
use tinyklv::prelude::*;            // Klv proc-macro, traits
use tinyklv::dec::binary as decb;   // binary decoders

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"HEARTBEAT",
    key(dec = decb::u8),
    len(dec = decb::u8_as_usize),
    default(typ = u8,  dec = decb::u8),
    default(typ = u16, dec = decb::be_u16),
    default(typ = u32, dec = decb::be_u32),
    allow_unimplemented_encode,
)]
/// Six-field heartbeat whose codecs are all resolved from container defaults
struct Heartbeat {
    #[klv(key = 0x01)] sequence:             u8,  // monotonic frame counter
    #[klv(key = 0x02)] temperature_centideg: u16, // temperature in 0.01 C units
    #[klv(key = 0x03)] battery_pct:          u8,  // battery charge 0..=100 %
    #[klv(key = 0x04)] rssi_dbm:             u8,  // receive signal strength (raw byte)
    #[klv(key = 0x05)] uptime_s:             u32, // seconds since boot
    #[klv(key = 0x06)] mode_flags:           u8,  // bitmask of active mode flags
}

fn main() {
    // manually construct the stream
    let stream = [
        0xDE, 0xAD, 0xBE, 0x00,             // junk preamble, no sentinel here
        // "HEARTBEAT" sentinel
        0x48, 0x45, 0x41, 0x52, 0x54, 0x42, 0x45, 0x41, 0x54,
        0x16,                               // body length = 22 bytes
        0x01, 0x01, 0x2A,                   // sequence          = 42
        0x02, 0x02, 0x09, 0x2E,             // temperature       = 2350
        0x03, 0x01, 0x57,                   // battery_pct       = 87
        0x04, 0x01, 0xB8,                   // rssi_dbm          = 0xB8
        0x05, 0x04, 0x00, 0x00, 0x0E, 0x10, // uptime_s          = 3600
        0x06, 0x01, 0x03,                   // mode_flags        = 0b0000_0011
    ];

    // manually construct the expected value
    let expected = Heartbeat {
        sequence:             42,
        temperature_centideg: 2350,
        battery_pct:          87,
        rssi_dbm:             0xB8,
        uptime_s:             3600,
        mode_flags:           0b0000_0011,
    };

    // seek sentinel, decode the value
    let decoded = Heartbeat::decode_frame(
        &mut stream.as_slice(),
    ).unwrap();

    // they equal!
    assert_eq!(decoded, expected);
}
