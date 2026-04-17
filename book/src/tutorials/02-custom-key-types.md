# 02 - Custom Key Types

The key codec is not hardcoded to `u8`. Any winnow parser that returns
some `K: Eq` plus an encoder that emits `Vec<u8>` works. This example
demonstrates three containers with different key types: `u8`,
big-endian `u16`, and little-endian `u16`.

## What it teaches

- Swapping `key(dec = ..., enc = ...)` at the container level
- Built-in codecs: `dec::binary::{be_u8, be_u16, le_u16}` /
  `enc::binary::{u8, be_u16, le_u16}`
- How endianness of the key codec changes the on-wire byte layout even
  when the Rust field types are identical

## The code

```rust,no_run
{{#include ../../../examples/02_custom_key_types.rs}}
```

## Run it

```sh
cargo run --example 02_custom_key_types
```

## Key takeaway

Key identity is a wire-format choice, not a Rust-type choice. Pick the
smallest key size that fits your namespace; widen only when you run out
of room.

Next: [03 - Strings and Primitives](./03-strings-and-types.md).
