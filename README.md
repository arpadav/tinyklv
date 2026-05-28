# tinyklv - the fastest KLV framework in Rust

[![Crates.io](https://img.shields.io/crates/v/tinyklv.svg)](https://crates.io/crates/tinyklv)
[![Documentation](https://img.shields.io/docsrs/tinyklv)](https://docs.rs/tinyklv)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![MSRV](https://img.shields.io/badge/rustc-1.95%2B-orange.svg)](https://www.rust-lang.org)

The fastest derive-macro framework for encoding and decoding [Key-Length-Value (KLV)](https://en.wikipedia.org/wiki/KLV)
binary streams, built on [`winnow`](https://crates.io/crates/winnow) parser combinators.

![Median per-call time across KLV frameworks](https://github.com/arpadav/tinyklv/blob/main/benches/bench.jpg?raw=true)

KLV (a generic Tag-Length-Value framing) is the backbone of telemetry packets,
video metadata streams, `IoT` sensor framing, and most custom binary protocols
that evolve without breaking older parsers. `tinyklv` is protocol-agnostic and
ships no baked-in standards: you declare your keys, length encoding, sentinel,
and per-field codecs as attributes on a struct, and `#[derive(Klv)]` generates
the encoder and decoder in a single pass.

## Quickstart

```sh
cargo add tinyklv
```

## Example

```rust
use tinyklv::Klv;
use tinyklv::prelude::*;
use tinyklv::dec::binary as decb;
use tinyklv::enc::binary as encb;

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"\x47\x48",
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize,
        enc = encb::u8_from_usize),
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
}

fn main() {
    let original = Heartbeat {
        sequence: 42,
        temperature_centideg: 2350,
    };
    let frame = original.encode_frame();
    let decoded = Heartbeat::decode_frame(
        &mut frame.as_slice()
    ).unwrap();
    assert_eq!(decoded, original);
}
```

Full annotated version: [`examples/01_hello_world.rs`](https://github.com/arpadav/tinyklv/blob/main/examples/01_hello_world.rs).

## Speed

Across decode and encode, flat and nested, clean and framed-over-noise,
`tinyklv` is the fastest derive-based KLV framework - several times faster than
`serde_klv`, an order of magnitude faster than `tlv_parser`, and within striking
distance of hand-rolled parsing. It is also the only one of the four that
handles **nested KLV** without hand-written glue.

The benchmark suite, the four competing implementations, and the one-command
chart reproduction (`benches/scripts/charts.sh`) all live in
[`benches/`](https://github.com/arpadav/tinyklv/tree/main/benches).

See [results here](https://github.com/arpadav/tinyklv/blob/main/benches/bench.jpg?raw=true)

## Maintainability

The only thing faster per packet is the `manual` bar - and it is the least
maintainable code of the four. A hand-rolled decoder is a panic-adjacent
slice-indexing loop with one `Option` per field to juggle, repeated for every
record shape, plus a second hand-written sub-parser for every level of nesting.
From the benchmark's `manual` nested decoder
([`benches/suite/approaches/manual/nested.rs`](https://github.com/arpadav/tinyklv/blob/main/benches/suite/approaches/manual/nested.rs)):

```rust
fn decode(body: &[u8]) -> Option<Platform> {
    let mut id = None;
    let mut coord = None;
    // ...one `let mut <field> = None;` for every one of nine fields...
    let mut sensors = None;
    let mut j = 0;
    while j + 2 <= body.len() { // bounds check
        let tag = body[j]; // allow clippy slice indexing
        let len = usize::from(body[j + 1]);
        j += 2;
        let val = body.get(j..j + len)?; // hand-rolled bounds math
        j += len;
        match tag { // manually define keys for each struct
            key::ID => id = Some(u32::from_be_bytes(val.try_into().ok()?)),
            key::COORD => coord = Some(decode_coord(val)?), // a SECOND hand-written loop
            // ...one arm per field, each with its own try_into().ok()? dance...
            _ => {}
        }
    }
    Some(Platform { id: id?, coord: coord?, /* ...unwrap all nine... */ sensors: sensors? })
}
```

`tinyklv` collapses the whole thing - both directions, nesting included - into
attributes on the struct:

```rust
#[derive(Klv)]
#[klv(
    stream = &[u8],
    sentinel = b"\x47\x48",
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
    trait_fallback,
)]
struct Platform {
    #[klv(key = 0x01, dec = decb::be_u32, enc = *encb::be_u32)]
    id: u32,
    
    #[klv(key = 0x02)]
    coord: GpsCoord,
    
    // ...the remaining fields, one attribute line each...
}
```

## Features

- Built-in codecs: binary (native/BE/LE for `u8`..`u128`, `i8`..`i128`, `f32`/`f64`), BER length, BER-OID keys, UTF-8 / UTF-16 / ASCII strings
- Sentinel seeking - resync on noisy byte streams
- Streaming partial packets on noisy or incomplete streams
- Repeated decode with user-defined break conditions
- Nested `Klv` structs - compose packets from sub-packets
- Generic structs and lifetimes, `Option<T>` fields, per-field/container defaults
- Stream type is user-selected, where any `winnow::Stream` works

## Documentation

- [API reference (docs.rs)](https://docs.rs/tinyklv)
- [Book (tutorials, reference, architecture)](https://arpadvoros.com/tinyklv)

## Contributing

Issues and pull requests welcome at
<https://github.com/arpadav/tinyklv>. Run `cargo test --all` and
`cargo clippy --all-targets -- -D warnings` before opening a PR.

## License

Licensed under the MIT License. See [LICENSE](LICENSE) for details.

## Support

If `tinyklv` is useful to you:

- [Buy Me a Coffee](https://buymeacoffee.com/arpadav)
- Bitcoin: `bc1q5stdywthj254agv80s5gky6440xy73cpqgv0q7`
