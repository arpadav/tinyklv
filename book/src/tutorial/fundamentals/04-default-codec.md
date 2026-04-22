# Tutorial 04 - Default codecs

`HeartbeatPacket` now carries six fields. Writing `dec = decb::u8` on every
`u8` field and `dec = decb::be_u16` on every `u16` field is pure ceremony -
the codec is determined by the type, not the field. Container-level
`default(typ = T, dec = ...)` attaches a codec to a concrete type; any field
of that type resolves its codec from the default and only needs `key = ...`.

Before this page each field attribute was two or three lines. After it, most
fields are one line. Fields that want a non-default codec still override it
locally; everything else stays terse and the intent stays obvious.

## Example

Run this example: `cargo run --example book_04_default_codec`

```rust,no_run
{{#include ../../../../examples/book_04_default_codec.rs}}
```

## Overview

- `default(typ = T, dec = f)` wires `f` for every field of type `T`.
- Field-level `dec = ...` still wins when a field needs something bespoke.
- One default per type, listed in the container attribute, not repeated.

**Next:** [05 - Custom decoder functions](./05-custom-decoder.md)
