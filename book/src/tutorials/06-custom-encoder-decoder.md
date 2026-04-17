# 06 - Custom Encoder/Decoder

Built-in codecs handle binary primitives and strings, but real protocols
often encode domain values with scaling factors, fixed-point
representations, or custom bit packing. This example shows how to write
your own encoder and decoder functions and hand them to the derive via
`dec =` and `enc =`.

## What it teaches

- The shape of a user-written decoder:
  `fn(&mut &[u8]) -> winnow::Result<T>`
- The shape of a user-written encoder:
  `fn(&T) -> Vec<u8>`
- Scaled-`f64` pattern: store as integer on the wire, divide on decode,
  multiply on encode

## The code

```rust,no_run
{{#include ../../../examples/06_custom_encoder_decoder.rs}}
```

## Run it

```sh
cargo run --example 06_custom_encoder_decoder
```

## Key takeaway

Custom codecs are just functions. No traits to implement, no registries
to populate. If you can write a `fn` with the right signature, you can
plug it into any field.

Next: [07 - Sentinel Seeking](./07-sentinel-seeking.md).
