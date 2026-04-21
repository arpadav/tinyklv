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
use tinyklv::dec::binary as dec;
use tinyklv::enc::binary as enc;

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"\x47\x48",
    key(dec = dec::be_u8, enc = enc::u8),
    len(dec = dec::be_u8_as_usize,
        enc = enc::u8_from_usize),
)]
struct HeartbeatPacket {
    #[klv(key = 0x01, dec = dec::be_u8,  enc = &enc::u8)]
    sequence: u8,
    #[klv(key = 0x02, dec = dec::be_u16, enc = &enc::be_u16)]
    temperature_centideg: u16,
}

fn main() {
    let original = HeartbeatPacket { sequence: 42, temperature_centideg: 2350 };

    let frame = original.encode_frame();
    let decoded = HeartbeatPacket::decode_frame(&mut frame.as_slice()).unwrap();

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

## Examples

Fifteen runnable examples live under [`examples/`](examples/). Run any with
`cargo run --example <name>`.

### Beginner

| # | File | What it shows |
|---|------|---------------|
| 01 | [`01_hello_world.rs`](examples/01_hello_world.rs) | Minimal two-field struct, full roundtrip |
| 02 | [`02_custom_key_types.rs`](examples/02_custom_key_types.rs) | Swap the key codec: `u8` vs BE-`u16` vs LE-`u16` |
| 03 | [`03_strings_and_types.rs`](examples/03_strings_and_types.rs) | UTF-8 strings alongside fixed-width integers |
| 04 | [`04_optional_fields.rs`](examples/04_optional_fields.rs) | `Option<T>` fields and missing-key handling |
| 05 | [`05_basic_roundtrip.rs`](examples/05_basic_roundtrip.rs) | BER-keyed, `encode_value` vs `encode_frame` |

### Intermediate

| # | File | What it shows |
|---|------|---------------|
| 06 | [`06_custom_encoder_decoder.rs`](examples/06_custom_encoder_decoder.rs) | User-written scaled-`f64` codec |
| 07 | [`07_sentinel_seeking.rs`](examples/07_sentinel_seeking.rs) | Recover from prefix noise using `seek_sentinel` |
| 08 | [`08_nested_packets.rs`](examples/08_nested_packets.rs) | Nested `Klv` structs as fields |
| 09 | [`09_ber_keyed.rs`](examples/09_ber_keyed.rs) | Multi-byte BER-OID keys |
| 10 | [`10_defaults_and_init.rs`](examples/10_defaults_and_init.rs) | Container-level and field-level defaults |

### Advanced

| # | File | What it shows |
|---|------|---------------|
| 11 | [`11_repeated_extraction.rs`](examples/11_repeated_extraction.rs) | Loop `decode_frame` over N concatenated frames |
| 12 | [`12_enum_dispatch_stream.rs`](examples/12_enum_dispatch_stream.rs) | Peek-dispatch heterogeneous packets into an enum |
| 13 | [`13_variable_length_fields.rs`](examples/13_variable_length_fields.rs) | Variable-width BER lengths |
| 14 | [`14_break_condition_custom.rs`](examples/14_break_condition_custom.rs) | Manual `DecodeValue` with skip/break logic |
| 15 | [`15_tokio_stream_e2e.rs`](examples/15_tokio_stream_e2e.rs) | End-to-end async pipeline with `tokio::mpsc` |

## Traits at a Glance

Everything ships through `tinyklv::prelude::*` (re-exports are anonymized via
`as _`, so the import does not pollute your namespace).

| Trait | Purpose | Typical call-site |
|-------|---------|-------------------|
| `EncodeValue<O>` | Encode only the value body (KLV triples, no frame header) | `val.encode_value()` |
| `EncodeFrame<O>` | Encode sentinel + length + value body | `val.encode_frame()` |
| `DecodeValue<S>` | Decode value body from an unframed slice | `T::decode_value(&mut s)` |
| `DecodeFrame<S>` | Seek sentinel, read length, subslice, then decode | `T::decode_frame(&mut s)` |
| `SeekSentinel<S>` | Advance the stream to the next sentinel occurrence | `T::seek_sentinel(&mut s)` |
| `RepeatedDecode<S>` | Decode a loop of N frames into a `Vec<T>` | internal to `repeated` fields |
| `BreakCondition<S>` | User-supplied stop predicate for repeated decode | advanced use |
| `IntoKlv<O>` | Marker for types emitted into the KLV output alphabet | codec authors |

## Attributes Cheat Sheet

### Container-level `#[klv(...)]`

| Attribute | Purpose |
|-----------|---------|
| `stream = &[u8]` | Input stream type (any `winnow::Stream`) |
| `sentinel = b"\xNN..."` | Magic bytes marking frame start - enables `decode_frame` / `SeekSentinel` |
| `key(dec = path, enc = path)` | Codec pair for the key field |
| `len(dec = path, enc = path)` | Codec pair for the length field (must produce `usize` on decode) |
| `default(typ = T, dec = path, enc = path)` | Default codec pair for every field of type `T` |
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
| `varlen` | Field has variable-width value (length prefix is authoritative) |
| `default = expr` | Value used if the key is absent on decode |
| `sentinel = b"..."` | Per-field sentinel for nested framed fields |
| `stream = &[u8]` | Per-field stream override |
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
