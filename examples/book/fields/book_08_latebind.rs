#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(clippy::unwrap_used)]
//! Book tutorial 08 - `latebind` post-decode hooks
//! See `book/tutorial/08-latebind.md` for the full narrative
//!
//! Demonstrates both forms of `latebind`:
//!
//! * **Consuming** (`latebind = Mode::from_u8`): the decoder returns a `u8`,
//!   the latebind function converts it to a `Mode` variant, and the struct
//!   field is declared as `Mode`
//! * **Mutating** (`latebind = &mut Coordinate::apply_global_z`): the decoder
//!   fills in `x` and `y`, and the latebind mutates `z` in place from a
//!   site-wide constant that is never transmitted over the wire
use tinyklv::prelude::*;            // Klv proc-macro + traits
use tinyklv::dec::binary as decb;   // binary decoders

#[derive(Debug, PartialEq, Clone, Copy)]
/// Operating mode, encoded as a single byte
enum Mode {
    Idle,
    Active,
    Error,
    Unknown,
}
impl Mode {
    /// Consuming latebind target: u8 value to `Mode`
    fn from_u8(v: u8) -> Self {
        match v {
            0 => Mode::Idle,
            1 => Mode::Active,
            2 => Mode::Error,
            _ => Mode::Unknown,
        }
    }
}

/// Site-wide altitude baseline, not transmitted per packet
const GLOBAL_Z: f32 = 15.0;

#[derive(Debug, PartialEq, Clone, Copy)]
/// 3D position. Only `x` and `y` are encoded; `z` is site-configured
struct Coordinate {
    x: f32,
    y: f32,
    z: f32,
}
impl Coordinate {
    /// Mutating latebind target: patch in the global altitude
    fn apply_global_z(&mut self) {
        self.z = GLOBAL_Z;
    }
}
impl DecodeValue<&[u8]> for Coordinate {
    /// Read x and y from the stream; leave z at zero for the latebind step
    fn decode_value(input: &mut &[u8]) -> tinyklv::Result<Self> {
        let x = decb::be_f32(input)?;
        let y = decb::be_f32(input)?;
        Ok(Coordinate { x, y, z: 0.0 })
    }
}

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
/// Heartbeat that uses both consuming and mutating `latebind` hooks on two fields
struct Heartbeat {
    #[klv(key = 0x01)]  sequence:             u8,  // monotonic frame counter
    #[klv(key = 0x02)]  temperature_centideg: u16, // temperature in 0.01 C units
    #[klv(key = 0x03)]  battery_pct:          u8,  // battery charge 0..=100 %
    #[klv(key = 0x04)]  rssi_dbm:             u8,  // receive signal strength
    #[klv(key = 0x05)]  uptime_s:             u32, // seconds since boot

    #[klv(
        key = 0x06,
        dec = decb::u8,
        latebind = Mode::from_u8,
    )]
    /// Decoded from `u8`, promoted to `Mode` by the consuming latebind
    mode: Mode,

    #[klv(
        key = 0x07,
        dec = Coordinate::decode_value,
        latebind = &mut Coordinate::apply_global_z,
    )]
    /// X and Y come from the stream; Z is injected by the mutating latebind
    position: Coordinate,
}

fn main() {
    // manually construct the stream
    let stream = [
        0xDE, 0xAD, 0xBE, 0x00,             // junk preamble, no sentinel here
        // "HEARTBEAT" sentinel
        0x48, 0x45, 0x41, 0x52, 0x54, 0x42, 0x45, 0x41, 0x54,
        0x20,                               // body length = 32 bytes
        0x01, 0x01, 0x2A,                   // sequence             = 42
        0x02, 0x02, 0x09, 0x2E,             // temperature_centideg = 2350
        0x03, 0x01, 0x57,                   // battery_pct          = 87
        0x04, 0x01, 0xB8,                   // rssi_dbm             = 0xB8
        0x05, 0x04, 0x00, 0x00, 0x0E, 0x10, // uptime_s             = 3600
        0x06, 0x01, 0x01,                   // mode u8              = 1 -> Mode::Active
        0x07, 0x08,                         // position: key, len = 8
        0x3F, 0xC0, 0x00, 0x00,             // x = 1.5
        0x40, 0x20, 0x00, 0x00,             // y = 2.5 (z injected by latebind)
    ];

    // manually construct the expected value
    let expected = Heartbeat {
        sequence:             42,
        temperature_centideg: 2350,
        battery_pct:          87,
        rssi_dbm:             0xB8,
        uptime_s:             3600,
        mode:                 Mode::Active,
        position:             Coordinate { x: 1.5, y: 2.5, z: GLOBAL_Z },
    };

    // seek sentinel, decode the value
    let decoded = Heartbeat::decode_frame(
        &mut stream.as_slice(),
    ).unwrap();

    // they equal!
    assert_eq!(decoded, expected);
}
