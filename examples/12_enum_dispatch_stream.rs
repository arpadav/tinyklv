//! Two packet types (Nav sentinel b"\xAA\x01", Weather sentinel b"\xAA\x02");
//! peek next 2 bytes, dispatch to correct decoder, build Vec<Packet> enum.
//!
//! Multiplexed telemetry buses often carry several packet types mixed together.
//! The canonical approach with tinyklv is to give each type its own sentinel,
//! then write a thin dispatcher that peeks the first two bytes and routes to
//! the appropriate `decode_frame`. This example mixes Nav and Weather frames
//! on a simulated bus, adds some inter-packet garbage bytes, and drains the
//! stream into a `Vec<TelemetryPacket>` enum, asserting the correct count and
//! order of decoded packets.

use tinyklv::prelude::*;
use tinyklv::Klv;

// --- Nav packet: sentinel 0xAA 0x01 ----------------------------------------

/// Navigation position and heading update.
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"\xAA\x01",
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct NavFrame {
    #[klv(key = 0x01, dec = tinyklv::dec::binary::be_f32, enc = &tinyklv::enc::binary::be_f32)]
    lat_deg: f32,
    #[klv(key = 0x02, dec = tinyklv::dec::binary::be_f32, enc = &tinyklv::enc::binary::be_f32)]
    lon_deg: f32,
    #[klv(key = 0x03, dec = tinyklv::dec::binary::be_u16, enc = &tinyklv::enc::binary::be_u16)]
    heading_centideg: u16,
}

// --- Weather packet: sentinel 0xAA 0x02 -------------------------------------

/// Atmospheric conditions snapshot.
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"\xAA\x02",
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct WeatherFrame {
    #[klv(key = 0x01, dec = tinyklv::dec::binary::be_u16, enc = &tinyklv::enc::binary::be_u16)]
    pressure_hpa: u16,
    #[klv(key = 0x02, dec = tinyklv::dec::binary::be_u8, enc = &tinyklv::enc::binary::u8)]
    humidity_pct: u8,
}

// --- Enum wrapper -----------------------------------------------------------

#[derive(Debug, PartialEq)]
enum TelemetryPacket {
    Nav(NavFrame),
    Weather(WeatherFrame),
}

/// Peek the next 2 bytes to identify the sentinel, then dispatch.
/// Advances past one byte if the sentinel is unknown, enabling re-sync.
fn dispatch_one(input: &mut &[u8]) -> Option<TelemetryPacket> {
    if input.len() < 2 {
        *input = &[];
        return None;
    }
    match &input[0..2] {
        b"\xAA\x01" => NavFrame::decode_frame(input).ok().map(TelemetryPacket::Nav),
        b"\xAA\x02" => WeatherFrame::decode_frame(input)
            .ok()
            .map(TelemetryPacket::Weather),
        _ => {
            // Unknown byte - advance one byte so the loop makes progress
            *input = &input[1..];
            None
        }
    }
}

fn main() {
    let nav1 = NavFrame {
        lat_deg: 51.5,
        lon_deg: -0.1,
        heading_centideg: 27_000,
    };
    let wx1 = WeatherFrame {
        pressure_hpa: 1013,
        humidity_pct: 65,
    };
    let nav2 = NavFrame {
        lat_deg: 51.6,
        lon_deg: -0.2,
        heading_centideg: 9_000,
    };
    let wx2 = WeatherFrame {
        pressure_hpa: 1011,
        humidity_pct: 72,
    };

    // Build a mixed stream: Nav, 3 garbage bytes, Weather, Nav, Weather
    let mut stream: Vec<u8> = nav1.encode_frame();
    stream.extend_from_slice(&[0xDE, 0xAD, 0xFF]); // inter-packet garbage
    stream.extend(wx1.encode_frame());
    stream.extend(nav2.encode_frame());
    stream.extend(wx2.encode_frame());

    println!("Mixed stream: {} bytes", stream.len());

    // Drain the stream using the dispatch loop
    let mut slice = stream.as_slice();
    let mut packets: Vec<TelemetryPacket> = Vec::new();
    while !slice.is_empty() {
        if let Some(p) = dispatch_one(&mut slice) {
            packets.push(p);
        }
    }

    println!("Decoded {} packets", packets.len());
    for (i, p) in packets.iter().enumerate() {
        match p {
            TelemetryPacket::Nav(n) => println!(
                "  [{}] Nav: lat={:.1}, lon={:.1}, hdg={}",
                i, n.lat_deg, n.lon_deg, n.heading_centideg
            ),
            TelemetryPacket::Weather(w) => println!(
                "  [{}] Weather: pressure={}hPa, humidity={}%",
                i, w.pressure_hpa, w.humidity_pct
            ),
        }
    }

    assert_eq!(packets.len(), 4);
    assert_eq!(packets[0], TelemetryPacket::Nav(nav1));
    assert_eq!(packets[1], TelemetryPacket::Weather(wx1));
    assert_eq!(packets[2], TelemetryPacket::Nav(nav2));
    assert_eq!(packets[3], TelemetryPacket::Weather(wx2));
    println!("SUCCESS");
}
