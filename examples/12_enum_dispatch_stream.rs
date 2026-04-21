#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(clippy::unwrap_used)]
//! Example 12 - variant dispatch on a multiplexed stream.
//!
//! Real telemetry buses carry several packet types mixed together. The
//! canonical tinyklv pattern is: give each packet type its own sentinel,
//! write a tiny peek-and-route dispatcher, and fold the results into a
//! `Vec<Packet>` enum. The dispatcher advances one byte at a time past
//! unknown prefixes so it can recover from inter-packet garbage.
//!
//! Showcases:
//! * Two `#[derive(Klv)]` structs with distinct sentinels
//! * A hand-written dispatcher that peeks the next few bytes
//! * Wrapping decoded frames in a plain Rust `enum`
//!
//! See also: book Tutorial 12.
use tinyklv::prelude::*;            // Klv proc-macro + traits
use tinyklv::dec::binary as decb;   // binary decoders
use tinyklv::enc::binary as encb;   // binary encoders

/// Sentinel for a navigation frame
const NAV_SENTINEL: &[u8] = b"NAVFRAME";
/// Sentinel for a weather frame
const WX_SENTINEL:  &[u8] = b"WXFRAME";

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],                                              // default, shown for clarity
    sentinel = b"NAVFRAME",
    key(dec = decb::u8,          enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
/// Navigation position and heading update
struct NavFrame {
    #[klv(
        key = 0x01,
        dec = decb::be_f32,
        enc = *encb::be_f32,
    )]
    /// Latitude in decimal degrees
    lat_deg: f32,

    #[klv(
        key = 0x02,
        dec = decb::be_f32,
        enc = *encb::be_f32,
    )]
    /// Longitude in decimal degrees
    lon_deg: f32,

    #[klv(
        key = 0x03,
        dec = decb::be_u16,
        enc = *encb::be_u16,
    )]
    /// Heading in 0.01 degree units
    heading_centideg: u16,
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],                                              // default, shown for clarity
    sentinel = b"WXFRAME",
    key(dec = decb::u8,          enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
/// Atmospheric conditions snapshot
struct WeatherFrame {
    /// Surface pressure in hPa
    #[klv(
        key = 0x01,
        dec = decb::be_u16,
        enc = *encb::be_u16,
    )]
    pressure_hpa: u16,

    /// Relative humidity in percent
    #[klv(
        key = 0x02,
        dec = decb::u8,
        enc = *encb::u8,
    )]
    humidity_pct: u8,
}

#[derive(Debug, PartialEq)]
/// Wrapper carrying whichever frame was found next on the bus
enum TelemetryPacket {
    Nav(NavFrame),
    Weather(WeatherFrame),
}

/// Peek the next few bytes, try each sentinel, dispatch to the right
/// `decode_frame`. Advances one byte on an unrecognised prefix so the loop
/// keeps making progress through inter-packet noise
fn dispatch_one(input: &mut &[u8]) -> Option<TelemetryPacket> {
    if input.starts_with(NAV_SENTINEL) {
        return NavFrame::decode_frame(input).ok().map(TelemetryPacket::Nav);
    }
    if input.starts_with(WX_SENTINEL) {
        return WeatherFrame::decode_frame(input).ok().map(TelemetryPacket::Weather);
    }
    if input.is_empty() {
        return None;
    }
    // unknown prefix - drop one byte and retry on the next call
    *input = &input[1..];
    None
}

fn main() {
    // build - two Nav and two Weather records to multiplex onto the bus
    let nav1 = NavFrame     { lat_deg: 51.5, lon_deg: -0.1, heading_centideg: 27_000 };
    let wx1  = WeatherFrame { pressure_hpa: 1013, humidity_pct: 65 };
    let nav2 = NavFrame     { lat_deg: 51.6, lon_deg: -0.2, heading_centideg:  9_000 };
    let wx2  = WeatherFrame { pressure_hpa: 1011, humidity_pct: 72 };

    // encode - concatenate four frames with three bytes of garbage spliced
    // between the first Nav and the first Weather to exercise the dispatcher
    let mut stream: Vec<u8> = Vec::new();
    stream.extend(nav1.encode_frame());
    // inter-packet garbage (not a sentinel prefix):
    stream.extend_from_slice(&[
        // three noise bytes:
            0xDE, 0xAD, 0xFF,
    ]);
    stream.extend(wx1.encode_frame());
    stream.extend(nav2.encode_frame());
    stream.extend(wx2.encode_frame());

    // decode - drain by peek-and-dispatch until the slice is empty
    let mut slice = stream.as_slice();
    let mut packets: Vec<TelemetryPacket> = Vec::new();
    while !slice.is_empty() {
        if let Some(p) = dispatch_one(&mut slice) {
            packets.push(p);
        }
    }

    // assert - exactly four packets recovered in the original order
    assert_eq!(packets.len(), 4);
    assert_eq!(packets[0], TelemetryPacket::Nav(nav1));
    assert_eq!(packets[1], TelemetryPacket::Weather(wx1));
    assert_eq!(packets[2], TelemetryPacket::Nav(nav2));
    assert_eq!(packets[3], TelemetryPacket::Weather(wx2));
}
