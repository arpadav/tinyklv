# Tutorial 04 - Default codecs

`Heartbeat` now carries six fields. Writing `dec = decb::u8` on every
`u8` field and `dec = decb::be_u16` on every `u16` field would be a lot of boiler-plate.
Container-level `default(typ = T, dec = ...)` attaches a default codec to a concrete
type, rather than it being field-specific. As a result, any field with type `T` which
does not have a `dec = ..` field attribute will fall back to the default type, only
needing `key = ..`!

Fields codecs will still take precidence over their defaults, if the same datatype (e.g.
and enumeration) is decoded in different ways.

## Example

Run this example: `cargo run --example book_04_default_codec`

```rust
{{#include ../../../../examples/book_04_default_codec.rs}}
```

## Overview

- `default(typ = T, dec = f)` uses `f` as a decoder for every key with value of type `T`.
- Field-level `dec = ...` still wins when a field needs something bespoke.
- One default per type, listed in the container attribute, not repeated.

**Next:** [05 - Custom decoder functions](./05-custom-decoder.md)
