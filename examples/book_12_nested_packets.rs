#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(clippy::unwrap_used)]
//! Book tutorial 12 - nested KLV packets
//!
//! Demonstrates composing an inner `GpsFix` sub-packet inside an outer
//! `Heartbeat` frame. The inner type derives `Klv` without a sentinel so it
//! can be embedded as the value region of a parent field; the outer type
//! references `GpsFix::decode_value` / `encode_value` directly in its `#[klv]`
//! attribute. Both directions are asserted to round-trip correctly, and the
//! inner type is also verified in isolation
//!
//! See `book/tutorial/12-nested-packets.md` for the full narrative.
//!
//! Author: aav
use tinyklv::prelude::*;            // Klv proc-macro + traits
use tinyklv::dec::binary as decb;   // binary decoders
use tinyklv::enc::binary as encb;   // binary encoders

#[derive(Klv, Debug, PartialEq, Clone, Copy)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8,          enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
/// Nested sub-packet
///
/// Sentinel **could** exist for stand-alone packet, however
/// in this example, it will never be seen as a top-level frame
struct GpsFix {
    #[klv(
        key = 0x01,
        dec = decb::be_i32,
        enc = *encb::be_i32
    )]
    /// Latitude in micro-degrees (i32)
    lat_udeg: i32,

    #[klv(
        key = 0x02,
        dec = decb::be_i32,
        enc = *encb::be_i32
    )]
    /// Longitude in micro-degrees (i32)
    lon_udeg: i32,

    #[klv(
        key = 0x03,
        dec = decb::u8,
        enc = *encb::u8
    )]
    /// Number of satellites used for the fix
    satellites: u8,
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"HEARTBEAT",
    key(dec = decb::u8,          enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
/// Top-level heartbeat frame with a nested `GpsFix`
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
        dec = GpsFix::decode_value,
        enc = GpsFix::encode_value,
    )]
    /// Nested `GpsFix`: `dec`/`enc` to the derived methods on the inner type
    gps: GpsFix,
}

fn main() {
    // build a heartbeat carrying a GPS fix
    let original = Heartbeat {
        sequence:             42,
        temperature_centideg: 2350,
        uptime_s:             3600,
        gps: GpsFix {
            lat_udeg:   51_477_200,   // ~51.477 N (London)
            lon_udeg:   -126_772,     // ~0.127 W
            satellites: 9,
        },
    };

    // encode - the GPS sub-packet occupies the value region of key 0x04 and
    // contains its own key/length/value triples inside
    let mut frame = Vec::new();
    original.encode_frame(&mut frame);

    // decode - outer and inner are reconstructed in one call
    let decoded = Heartbeat::decode_frame(
        &mut frame.as_slice(),
    ).unwrap();
    assert_eq!(decoded, original);

    // the inner type can also be used on its own, which is how we assert it
    // round-trips independently of the outer frame
    let mut inner_bytes = Vec::new();
    original.gps.encode_value(&mut inner_bytes);
    let inner_decoded = GpsFix::decode_value(
        &mut inner_bytes.as_slice(),
    ).unwrap();
    assert_eq!(inner_decoded, original.gps);
}
