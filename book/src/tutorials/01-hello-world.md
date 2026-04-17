# 01 - Hello World

The minimal viable tinyklv packet: one container, two fields, a full
encode → decode → `assert_eq!` roundtrip.

## What it teaches

- The anatomy of a `#[derive(Klv)]` container attribute
- Why every field needs both `dec =` and `enc =` paths
- The difference between `encode_frame` and `encode_value` (the former
  wraps the body in `sentinel + length`; the latter emits the KLV triples
  only)

## The code

```rust,no_run
{{#include ../../../examples/01_hello_world.rs}}
```

## Run it

```sh
cargo run --example 01_hello_world
```

## Key takeaways

1. The container attribute `#[klv(...)]` declares *how* to frame the struct:
   stream type, sentinel bytes, key codec pair, length codec pair.
2. Each field attribute declares *where* its value lives on the wire: the
   key, the decoder that reads it, the encoder that writes it.
3. `encode_frame` gives you a complete on-the-wire packet; `decode_frame`
   consumes one and produces a typed struct. The roundtrip is exact.

Next: [02 - Custom Key Types](./02-custom-key-types.md).
