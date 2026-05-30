#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(clippy::unwrap_used)]
//! Book tutorial 07b - `varlen = true` for length-parameterised value decoders
//! See `book/tutorial/07-val-lengths.md` for the full narrative
//!
//! Shows how to add a variable-length `String` field to an existing struct
//! The `varlen = true` annotation tells the generated loop to pass the decoded
//! length into `decs::to_string_utf8` so it reads exactly that many bytes -
//! the standard fixed-width `fn(&mut Stream) -> Result<T>` signature cannot
//! do this on its own
use tinyklv::prelude::*;            // Klv proc-macro + traits
use tinyklv::dec::binary as decb;   // binary decoders
use tinyklv::dec::string as decs;   // string decoders

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
/// Heartbeat extended with a variable-length `station_id` UTF-8 string field
struct Heartbeat {
    #[klv(key = 0x01)]  sequence:             u8,     // monotonic frame counter
    #[klv(key = 0x02)]  temperature_centideg: u16,    // temperature in 0.01 C units
    #[klv(key = 0x03)]  battery_pct:          u8,     // battery charge 0..=100 %
    #[klv(key = 0x04)]  rssi_dbm:             u8,     // receive signal strength
    #[klv(key = 0x05)]  uptime_s:             u32,    // seconds since boot
    #[klv(key = 0x06)]  mode_flags:           u8,     // bitmask of active flags

    #[klv(
        key = 0x07,
        dec = decs::to_string_utf8,
        varlen = true,
    )]
    /// Variable-length UTF-8 station identifier; `varlen = true` threads the
    /// decoded length into the string decoder
    station_id: String,
}

fn main() {
    // manually construct the stream
    let stream = [
        0xDE, 0xAD, 0xBE, 0x00,                         // junk preamble, no sentinel here
        // "HEARTBEAT" sentinel
        0x48, 0x45, 0x41, 0x52, 0x54, 0x42, 0x45, 0x41, 0x54,
        0x20,                                           // body length = 32 bytes
        0x01, 0x01, 0x2A,                               // sequence             = 42
        0x02, 0x02, 0x09, 0x2E,                         // temperature_centideg = 2350
        0x03, 0x01, 0x57,                               // battery_pct          = 87
        0x04, 0x01, 0xB8,                               // rssi_dbm             = 0xB8
        0x05, 0x04, 0x00, 0x00, 0x0E, 0x10,             // uptime_s             = 3600
        0x06, 0x01, 0x03,                               // mode_flags           = 0b0000_0011
        0x07, 0x08,                                     // station_id: key, len = 8
        0x53, 0x45, 0x4E, 0x53, 0x4F, 0x52, 0x2D, 0x37, // station_id value = "SENSOR-7"
    ];

    // manually construct the expected value
    let expected = Heartbeat {
        sequence:             42,
        temperature_centideg: 2350,
        battery_pct:          87,
        rssi_dbm:             0xB8,
        uptime_s:             3600,
        mode_flags:           0b0000_0011,
        station_id:           String::from("SENSOR-7"),
    };

    // seek sentinel, decode the value
    let decoded = Heartbeat::decode_frame(
        &mut stream.as_slice(),
    ).unwrap();

    // they equal!
    assert_eq!(decoded, expected);
}
