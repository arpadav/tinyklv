# Getting Started

## Install

```sh
cargo add tinyklv
```

`tinyklv` targets Rust 1.95+ and re-exports [`winnow`](https://crates.io/crates/winnow)
machinery under the `tinyklv::__export` namespace. You do not import `winnow`
directly unless you are writing a custom codec.

## First packet

```rust
use tinyklv::Klv;
use tinyklv::prelude::*;

fn enc_u8(v: &u8)   -> Vec<u8> { tinyklv::enc::binary::u8(*v) }
fn enc_u16(v: &u16) -> Vec<u8> { tinyklv::enc::binary::be_u16(*v) }

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"\x47\x48",
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize,
        enc = tinyklv::enc::binary::u8_from_usize),
)]
struct HeartbeatPacket {
    #[klv(key = 0x01, dec = tinyklv::dec::binary::be_u8,  enc = enc_u8)]
    sequence: u8,
    #[klv(key = 0x02, dec = tinyklv::dec::binary::be_u16, enc = enc_u16)]
    temperature_centideg: u16,
}

fn main() {
    let original = HeartbeatPacket { sequence: 42, temperature_centideg: 2350 };

    let frame = original.encode_frame();
    let decoded = HeartbeatPacket::decode_frame(&mut frame.as_slice()).unwrap();

    assert_eq!(decoded, original);
}
```

## What the macro generated

Four trait impls are emitted:

| Trait | Method | Purpose |
|-------|--------|---------|
| `EncodeValue<Vec<u8>>` | `.encode_value()` | Emits the KLV triples only |
| `EncodeFrame<Vec<u8>>` | `.encode_frame()` | Sentinel + length + value body |
| `DecodeValue<&[u8]>` | `decode_value(&mut s)` | Parses triples from a slice |
| `DecodeFrame<&[u8]>` | `decode_frame(&mut s)` | Seeks sentinel, reads length, decodes |

Any of the four can be opted out via `allow_unimplemented_encode` /
`allow_unimplemented_decode` on the container.

## Next step

Continue to [What is KLV?](./what-is-klv.md) for the framing primer, or jump
straight into the [tutorials](./tutorials/beginner.md).
