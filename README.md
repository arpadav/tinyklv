# tinyklv: A [Key-Length-Value (KLV)](https://en.wikipedia.org/wiki/KLV) framework in Rust using [`winnow`](https://crates.io/crates/winnow)

[![LICENSE](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Crates.io Version](https://img.shields.io/crates/v/tinyklv.svg)](https://crates.io/crates/tinyklv)

`tinyklv` is a derive-macro framework for encoding and decoding KLV (Key-Length-Value)
binary streams. Define your packet struct, annotate with `#[derive(Klv)]`, and get
`Decode`, `Encode`, `Seek`, and `Extract` implementations generated automatically.

Built on [`winnow`](https://crates.io/crates/winnow) for parsing. Designed for network
packets, telemetry streams (MISB 0601/0903), and any TLV-structured binary protocol.

## Basic

A struct with primitive fields. Decode a raw KLV stream, encode it back.

```rust,ignore
use tinyklv::Klv;
use tinyklv::prelude::*;

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct BasicPacket {
    #[klv(key = 0x01, var = true, dec = tinyklv::dec::binary::to_string_utf8, enc = tinyklv::enc::string::from_string_utf8)]
    label: String,
    #[klv(key = 0x02, dec = tinyklv::dec::binary::be_u16, enc = tinyklv::enc::binary::be_u16)]
    value: u16,
}

// Decode from raw bytes
let stream: &[u8] = &[
    0x01, 0x03, 0x4B, 0x4C, 0x56,  // key=0x01, len=3, "KLV"
    0x02, 0x02, 0x01, 0x02,        // key=0x02, len=2, 258
];
let packet = BasicPacket::decode(&mut &*stream).unwrap();
assert_eq!(packet.label, "KLV");
assert_eq!(packet.value, 258);

// Encode back to bytes — roundtrip identity
let encoded = packet.encode_value();
let roundtrip = BasicPacket::decode(&mut encoded.as_slice()).unwrap();
assert_eq!(roundtrip, packet);
```

## Intermediate

Custom domain types, sentinel-based stream seeking, and optional fields.
Each custom type implements `Decode` and `EncodeValue` manually — the derive
macro calls your encoder/decoder functions per field.

```rust,ignore
use tinyklv::Klv;
use tinyklv::prelude::*;

// -- Custom types with manual encode/decode --

#[derive(Debug, Clone, Copy, PartialEq)]
enum Priority { Low, Medium, High, Critical }

fn decode_priority(input: &mut &[u8]) -> winnow::Result<Priority> {
    let b = tinyklv::dec::binary::be_u8(input)?;
    match b {
        0 => Ok(Priority::Low),
        1 => Ok(Priority::Medium),
        2 => Ok(Priority::High),
        3 => Ok(Priority::Critical),
        _ => Err(winnow::error::ParserError::from_input(input)),
    }
}
fn encode_priority(v: &Priority) -> Vec<u8> {
    vec![match v { Priority::Low => 0, Priority::Medium => 1, Priority::High => 2, Priority::Critical => 3 }]
}

#[derive(Debug, Clone, PartialEq)]
struct Coordinate { lat: f64, lon: f64 }

fn decode_coordinate(input: &mut &[u8]) -> winnow::Result<Coordinate> {
    Ok(Coordinate {
        lat: tinyklv::dec::binary::be_f64(input)?,
        lon: tinyklv::dec::binary::be_f64(input)?,
    })
}
fn encode_coordinate(v: &Coordinate) -> Vec<u8> {
    [tinyklv::enc::binary::be_f64(v.lat), tinyklv::enc::binary::be_f64(v.lon)].concat()
}

// -- Sentinel packet: seek + decode in one step --

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"\xBE\xEF",
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct NavPacket {
    #[klv(key = 0x01, dec = decode_coordinate, enc = encode_coordinate)]
    position: Coordinate,
    #[klv(key = 0x02, dec = decode_priority, enc = encode_priority)]
    priority: Priority,
    #[klv(key = 0x03, dec = decode_coordinate, enc = encode_coordinate)]
    destination: Option<Coordinate>,
}

let packet = NavPacket {
    position:    Coordinate { lat: 37.7749, lon: -122.4194 },
    priority:    Priority::High,
    destination: None,  // optional — omitted from encoding
};

// encode() prepends sentinel + length automatically
let encoded = packet.encode();
assert_eq!(&encoded[0..2], b"\xBE\xEF");

// extract() seeks the sentinel in any stream, then decodes
let mut stream = encoded.as_slice();
let decoded = NavPacket::extract(&mut stream).unwrap();
assert_eq!(decoded, packet);
assert_eq!(decoded.destination, None);

// Garbage bytes before the sentinel are skipped
let mut noisy = vec![0xDE, 0xAD, 0xFF];
noisy.extend(&encoded);
let decoded = NavPacket::extract(&mut noisy.as_slice()).unwrap();
assert_eq!(decoded.position.lat, 37.7749);
```

## Advanced

Multiple packet types on one stream. Each has a unique sentinel. A thin
enum wrapper peeks the sentinel bytes and dispatches to the correct
`extract()` call — the derive macro handles everything else.

```rust,ignore
use tinyklv::Klv;
use tinyklv::prelude::*;

// NavPacket (sentinel 0xBEEF) and WeatherPacket (sentinel 0xCAFE)
// defined with #[derive(Klv)] as in the Intermediate example...

#[derive(Debug)]
enum Packet {
    Nav(NavPacket),
    Weather(WeatherPacket),
}

/// Peek 2 bytes, dispatch to the matching extract()
fn dispatch(input: &mut &[u8]) -> Option<Packet> {
    if input.len() < 2 {
        *input = &[];
        return None;
    }
    match &input[0..2] {
        b"\xBE\xEF" => NavPacket::extract(input).ok().map(Packet::Nav),
        b"\xCA\xFE" => WeatherPacket::extract(input).ok().map(Packet::Weather),
        _ => { *input = &input[1..]; None } // skip unknown byte
    }
}

// Build a mixed stream: Nav, Weather, Nav
let mut stream: Vec<u8> = Vec::new();
stream.extend(nav1.encode());
stream.extend(weather1.encode());
stream.extend(nav2.encode());

// Drain the stream
let mut slice = stream.as_slice();
let mut packets = Vec::new();
while !slice.is_empty() {
    if let Some(p) = dispatch(&mut slice) {
        packets.push(p);
    }
}
assert_eq!(packets.len(), 3);
```

### Extracting repeated packets

For a single packet type, loop `extract()` directly:

```rust,ignore
let stream: Vec<u8> = waypoints.iter().flat_map(|w| w.encode()).collect();
let mut slice = stream.as_slice();
let mut results = Vec::new();
while let Ok(w) = Waypoint::extract(&mut slice) {
    results.push(w);
}
assert_eq!(results.len(), waypoints.len());
```

### Owned-value encoders

Encoder functions receive `&T` by default. If your encoder takes `T` by value,
wrap it with `enc_owned!`:

```rust,ignore
fn encode_priority_owned(v: Priority) -> Vec<u8> { /* ... */ }

#[derive(Klv)]
#[klv(/* ... */)]
struct Packet {
    #[klv(key = 0x01, dec = decode_priority, enc = tinyklv::enc_owned!(encode_priority_owned))]
    priority: Priority,
}
```

## Features

| Feature | Description |
|---------|-------------|
| `full` | Enables all optional features |
| `ascii` | ASCII string codec |
| `chrono` | Date/time parsing via `chrono` |
| `tracing` | Debug logging during decode |

## Key Concepts

| Trait | Direction | What it does |
|-------|-----------|--------------|
| `Decode<S>` | Decode | Parse fields from a KLV byte stream |
| `EncodeValue<O>` | Encode | Serialize fields to KLV bytes |
| `Seek<S>` | Decode | Find a packet by its sentinel |
| `Extract<S>` | Decode | `Seek` + `Decode` in one step |
| `Encode<O>` | Encode | Sentinel + length + `EncodeValue` |

- **`decode()`** — parse KLV triples from a byte stream (no sentinel)
- **`extract()`** — seek sentinel, then decode (use for framed streams)
- **`encode_value()`** — serialize fields as KLV triples
- **`encode()`** — sentinel + length prefix + `encode_value()` (for sentinel types)

## License

`tinyklv` is licensed under the [MIT License](./LICENSE).
