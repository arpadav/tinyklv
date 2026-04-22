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
via attributes on a struct. The derive macro generates the wire format
accessors; you control the schema.

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
    key(dec = decb::be_u8, enc = encb::u8),
    len(dec = decb::be_u8_as_usize,
        enc = encb::u8_from_usize),
)]
struct HeartbeatPacket {
    #[klv(
        key = 0x01,
        dec = decb::be_u8,
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
    let original = HeartbeatPacket {
        sequence: 42,
        temperature_centideg: 2350,
    };
    let frame = original.encode_frame();
    let decoded = HeartbeatPacket::decode_frame(
        &mut frame.as_slice()
    ).unwrap();
    assert_eq!(decoded, original);
}
```

Full annotated version: [`examples/01_hello_world.rs`](examples/01_hello_world.rs).

## Feature Highlights

- `#[derive(Klv)]` proc-macro generates encode and decode in one pass
- Built-in codecs: binary (BE/LE for `u8..u64`, `i8..i64`, `f32/f64`), BER length, BER-OID keys, UTF-8 / UTF-16 / ASCII strings
- Sentinel seeking - resync on noisy byte streams via `seek_sentinel`
- Repeated decode with user-defined break conditions
- Nested `Klv` structs - compose packets from sub-packets
- Generic structs and lifetimes supported (see below)
- `Option<T>` fields, per-field and per-container defaults, `deny_unknown_keys`
- Stream type is user-selected (`&[u8]` is the default but not required) - any `winnow::Stream` works

## Traits at a Glance

Everything ships through `tinyklv::prelude::*`.

| Trait | Purpose | Typical call-site |
|-------|---------|-------------------|
| `EncodeValue<O>` | Encode only the value body (KLV triples, no frame header) | `val.encode_value()` |
| `EncodeFrame<O>` | Encode sentinel + length + value body | `val.encode_frame()` |
| `DecodeValue<S>` | Decode value body from an unframed slice | `T::decode_value(&mut s)` |
| `DecodeFrame<S>` | Seek sentinel, read length, subslice, then decode | `T::decode_frame(&mut s)` |
| `SeekSentinel<S>` | Seek past next sentinel+length, return the body sub-slice | `T::seek_sentinel(&mut s)` |
| `RepeatedDecode<S>` | Decode a repeating stream of `T` into a `Vec<T>` via `T::repeated(&mut s)` | blanket impl on `DecodeValue` |
| `BreakCondition<S>` | User-supplied per-`(key, len)` stop predicate consulted inside derive-generated `decode_value` loops | advanced use |
| `IntoKlv<O>` | Marker for types emitted into the KLV output alphabet | codec authors |

## Attributes Cheat Sheet

### Container-level `#[klv(...)]`

| Attribute | Purpose |
|-----------|---------|
| `stream = &[u8]` | Input stream type (any `winnow::Stream`) |
| `sentinel = b"\xNN..."` | Magic bytes marking frame start - enables `decode_frame` / `SeekSentinel` |
| `key(dec = path, enc = path)` | Codec pair for the key field |
| `len(dec = path, enc = path)` | Codec pair for the length field (must produce `usize` on decode) |
| `default(typ = T, dec = path, enc = path, varlen = <bool>)` | Default codec pair for every field of type `T` (`dec`, `enc`, `varlen` all optional) |
| `debug` | Emit the generated impl blocks at compile time |
| `deny_unknown_keys` | Error on unrecognized keys instead of skipping |
| `allow_unimplemented_encode` | Skip generating `EncodeValue`/`EncodeFrame` |
| `allow_unimplemented_decode` | Skip generating `DecodeValue`/`DecodeFrame` |

### Field-level `#[klv(...)]`

| Attribute | Purpose |
|-----------|---------|
| `key = 0xNN` | Key value for this field |
| `dec = path` | Decoder function `fn(&mut S) -> winnow::Result<T>` |
| `enc = path` | Encoder taking `&T` → emits `enc(&self.field)` (deref coercion covers `&String → &str`, `&Vec<u8> → &[u8]`) |
| `enc = &path` | `EncodeAs`-dispatched: primitives pass by value (Copy), `String → &str`, `Vec<T> → &[T]`, `Box/Rc/Arc<T> → &T`. No clone, no alloc. |
| `enc = *path` | Encoder taking `T` by value → emits `enc(self.field)` (for `Copy` primitives) |
| `varlen = <bool>` | Field has variable-width value (length prefix is authoritative) |
| `default` | Fallback via `Default::default()` when the key is absent on decode |
| `default = <expr>` | Fallback expression when the key is absent on decode |
| `latebind` | Post-decode conversion or mutation. `latebind = path` consumes (`Fn(T) -> U`); `latebind = &mut path` mutates in place (`Fn(&mut T)`). |

## Generic Structs

`#[derive(Klv)]` preserves generics and lifetimes verbatim via
`split_for_impl()`, so you can do:

```rust,ignore
#[derive(Klv)]
#[klv(/* ... */)]
struct Packet<'a, T: MyBound> {
    #[klv(key = 0x01, dec = ..., enc = ...)]
    payload: T,
    _marker: std::marker::PhantomData<&'a T>,
}
```

Bounded type parameters, lifetimes, and `PhantomData` all compose cleanly with
the generated `EncodeValue` / `DecodeFrame` / friends. See
[`tests/derive/advanced_generics.rs`](tests/derive/advanced_generics.rs) for
the canonical reference.

## mdBook Documentation

Long-form conceptual docs (how the macro expands, codec authoring, performance
notes) are being scaffolded. Placeholder: <https://arpadav.github.io/tinyklv/>.

## Contributing

Issues and pull requests are welcome at
<https://github.com/arpadav/tinyklv>. Run `cargo test --all` and
`cargo clippy --all-targets -- -D warnings` before opening a PR.

## Changelog

See [releases](https://github.com/arpadav/tinyklv/releases).

## License

Licensed under the MIT License. See [LICENSE](LICENSE) for details.
