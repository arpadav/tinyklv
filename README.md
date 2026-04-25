# tinyklv - KLV framework in Rust

[![Crates.io](https://img.shields.io/crates/v/tinyklv.svg)](https://crates.io/crates/tinyklv)
[![Documentation](https://img.shields.io/docsrs/tinyklv)](https://docs.rs/tinyklv)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![MSRV](https://img.shields.io/badge/rustc-1.95%2B-orange.svg)](https://www.rust-lang.org)
[![CI](https://img.shields.io/github/actions/workflow/status/arpadav/tinyklv/ci.yml?branch=main)](https://github.com/arpadav/tinyklv/actions)

A derive-macro framework for encoding and decoding [Key-Length-Value (KLV)](https://en.wikipedia.org/wiki/KLV)
binary streams, built on [`winnow`](https://crates.io/crates/winnow) parser combinators.

## What is KLV?

KLV is a generic Tag-Length-Value (TLV) framing pattern: every field in a byte
stream is prefixed by a key identifying it and a length giving its size. It is
the backbone of telemetry packets, video metadata streams, IoT sensor framing,
and most custom binary protocols that need to evolve without breaking older
parsers.

`tinyklv` is protocol-agnostic. It ships no baked-in standards - you declare
your keys, your length encoding, your sentinel, and the codec for each field
via attributes on a struct. The derive macro generates the encoder/decoder trait
implementations; you control the schema.

## Quick Start

```sh
cargo add tinyklv
```

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

Full annotated version: [`examples/01_hello_world.rs`](examples/01_hello_world.rs).

### Streaming / partial decode

For fragmented transports where a single packet may straddle multiple
reads, `Decoder<T>` owns an internal buffer and reassembles packets
automatically. Requires `sentinel = ...` on the struct.

```rust,ignore
let mut dec = Heartbeat::decoder();
loop {
    let n = socket.recv(&mut buf)?;
    dec.feed(&buf[..n]);
    for pkt in dec.by_ref() {
        handle(pkt?);
    }
}
```

Uses iterator patterns - `Decoder` implements `Iterator`, so `for pkt in dec.by_ref()` drains all available packets after each feed.

## Feature Highlights

- `#[derive(Klv)]` generates encode and decode in one pass
- Built-in codecs: binary (native/BE/LE for `u8`..`u128`, `i8`..`i128`, `f32`/`f64`), BER length, BER-OID keys, UTF-8 / UTF-16 / ASCII strings
- Sentinel seeking - resync on noisy byte streams
- Streaming `Decoder<T>` - feed/next API for fragmented transports (TCP, UDP, ring buffers)
- Repeated decode with user-defined break conditions
- Nested `Klv` structs - compose packets from sub-packets
- Generic structs and lifetimes supported
- `Option<T>` fields, per-field and per-container defaults, `trait_fallback`, `deny_unknown_keys`
- Stream type is user-selected - any `winnow::Stream` works

## Traits

| Trait | Purpose |
|-------|---------|
| `DecodeValue<S>` | Decode value body from an unframed slice |
| `DecodeFrame<S>` | Seek sentinel, read length, subslice, then decode |
| `EncodeValue<O>` | Encode the value body (KLV triples, no frame header) |
| `EncodeFrame<O>` | Encode sentinel + length + value body |
| `DrainFrames<S>` | Decode a sentinel-framed stream into `Vec<T>` |
| `DecodePartial<S>` | Streaming-aware decode returning `Packet<T, P>` |
| `Decoder<P, S>` | Owned-buffer streaming decoder with feed / next |
| `BreakCondition<S>` | Per-`(key, len)` stop predicate for decode loops |

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
