# 08 - Nested Packets

A KLV packet can carry another KLV packet as the value of a field.
`tinyklv` composes recursively: derive `Klv` on the inner struct, then
point the outer field's `dec` and `enc` at `Inner::decode_value` and
`Inner::encode_value`.

## What it teaches

- Using `Struct::decode_value` and `Struct::encode_value` as codec paths
- Why nested containers typically skip the inner sentinel
- How the outer length field still governs the inner bytes

## The code

```rust,no_run
{{#include ../../../examples/08_nested_packets.rs}}
```

## Run it

```sh
cargo run --example 08_nested_packets
```

## Key takeaway

A tinyklv-derived struct *is* a codec. Any field whose type also derives
`Klv` can drop the struct's own `decode_value`/`encode_value` methods
straight into the `dec`/`enc` slots of the outer field.

Next: [09 - BER-Keyed Containers](./09-ber-keyed.md).
