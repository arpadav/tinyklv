# Tutorial 08 - Latebind transforms

`latebind` runs a function *after* the field decoder. It comes in two shapes
distinguished by a sigil.

## Consuming

* Notation: `latebind = path`
* `path` signature: `Fn(T) -> U`

The decoder reads a raw `T` from the wire; `latebind` promotes it to the field's declared type `U`. The example wires `mode: Mode` with `dec = decb::u8` plus `latebind = Mode::from_u8`.

## Mutating

* Notation: `latebind = &mut path`
* `path` signature: `Fn(&mut T)`

The field stays type `T`; the function patches it in place. The example decodes only `x` and `y` for `position: Coordinate`, then `Coordinate::apply_global_z` fills in the `z` from the `GLOBAL_Z` constant.

## Example

```rust,no_run
{{#include ../../../examples/book_08_latebind.rs}}
```

## Overview

- Consuming `latebind` promotes a raw wire type into a richer Rust type.
- Mutating `latebind` injects context the wire format does not carry.
- Both shapes run after the field decoder; both are field-level only.

**Next:** [09 - Encoding & the `&` sigil](./09-encode-sigil.md)
