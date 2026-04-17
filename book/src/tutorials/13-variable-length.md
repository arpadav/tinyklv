# 13 - Variable-Length Fields

Fixed 1-byte lengths cap values at 255 bytes. Real payloads - JPEG
thumbnails, long strings, nested KLV - routinely exceed that. BER
length encoding handles values up to `2^31 - 1` bytes in a few bytes of
overhead.

## What it teaches

- `tinyklv::dec::ber::ber_length_decode` /
  `tinyklv::enc::ber::ber_length_encode`
- Mixing fixed-width key (`u8`) with BER length on the same container
- Why the BER short-form (`< 128`) encodes in 1 byte, same as a plain
  `u8`

## The code

```rust,no_run
{{#include ../../../examples/13_variable_length_fields.rs}}
```

## Run it

```sh
cargo run --example 13_variable_length_fields
```

## Key takeaway

Pick your length encoding to match your maximum payload size. BER is
the default for most real-world protocols; fixed `u8_as_usize` is fine
for compact, bounded schemas.

Next: [14 - Custom Break Conditions](./14-break-condition.md).
