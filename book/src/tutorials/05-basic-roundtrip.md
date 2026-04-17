# 05 - Basic Roundtrip

A deliberate tour of all four generated methods on the same struct:
`encode_value`, `encode_frame`, `decode_value`, `decode_frame`. Useful
when you need to splice KLV bodies into an outer wrapper that already
carries sentinel+length.

## What it teaches

- The boundary between **value** (KLV triples only) and **frame**
  (sentinel + length + triples)
- BER-encoded length field
- Why you might prefer `encode_value` when the outer transport handles
  framing itself

## The code

```rust,no_run
{{#include ../../../examples/05_basic_roundtrip.rs}}
```

## Run it

```sh
cargo run --example 05_basic_roundtrip
```

## Key takeaway

If your transport already frames messages (e.g., a TCP length-prefixed
channel), reach for `encode_value` / `decode_value` and keep the
sentinel out of the wire. If your transport is raw bytes (UDP, serial,
MPEG-TS private data), reach for `encode_frame` / `decode_frame`.

Next: [06 - Custom Encoder/Decoder](./06-custom-encoder-decoder.md).
