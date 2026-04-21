# Tutorial 01 - First packet

`HeartbeatPacket` carries two fields: a one-byte `sequence` and a two-byte big-endian
`temperature_centideg`. The stream is hand-built so you can see every byte:
a key, a length, and the value - repeated twice, back to back.

The container sets `key(dec = decb::u8)` and `len(dec = decb::u8_as_usize)` to
say "one byte for the key, one byte for the length". Each field picks its own
value decoder (`decb::u8`, `decb::be_u16`). Decoder function signatures are discussed
more in the [Implementing Custom Decoder](./05-custom-decoder.md) tutorial.

Decoding is driven by `DecodeValue::decode_value`, which takes `&mut &[u8]` and advances the slice pointer in place.

In addition, you might see `allow_unimplemented_encode` here. This is a flag, where by
default, `tinyklv::Klv` requires both encoding and decoding to be implemented. 
[More on this here](../reference/container-attributes.md#allow_unimplemented_encode)

## Example

Run this example: `cargo run --example book_01_getting_started`

```rust,no_run
{{#include ../../../examples/book_01_getting_started.rs}}
```

## Overview

- KLV triples are `key | length | value`; the derive wires them together.
- `decode_value` consumes the slice in place. `DecodeValue` trait provides
this, and is imported from the prelude.
- `allow_unimplemented_encode` is a flag, which is used in this tutorial until
encoding is covered.

**Next:** [02 - Out-of-order & prelude](./02-out-of-order.md)
