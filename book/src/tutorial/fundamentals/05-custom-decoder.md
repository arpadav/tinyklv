# Tutorial 05 - Custom decoder functions

Every path passed to `dec = ...` obeys the same contract:

```text
fn <name>(input: &mut S) -> tinyklv::Result<T>;
```

where `S` is the stream type (here `&[u8]`) and `T` is the field type.
`decb::u8`, `decb::be_u16`, and friends are just functions written to this
signature - nothing stops you from writing one.

Here the wire format stays the same (two-byte big-endian centidegrees) but
the field type becomes a domain-specific `Celsius(f32)`. The free-standing
`decode_celsius` function reads the `u16` and scales it. Because the field's
type is `Celsius`, the container's `default(typ = u16, ...)` no longer
applies - so the field attribute carries its own `dec = decode_celsius`.

## Example

Run this example: `cargo run --example book_05_custom_decoder_fn`

```rust,no_run
{{#include ../../../../examples/book_05_custom_decoder_fn.rs}}
```

## Overview

- `fn(&mut S) -> tinyklv::Result<T>` is the only contract - built-ins follow it.
- Domain types get their decoder by wiring a bespoke function into `dec = ...`.
- Container defaults match on type; changing the field type opts out cleanly.

**Next:** [06 - Implementing DecodeValue](./06-decode-value-impl.md)
