# Tutorial 09 - Encoding & the `&` sigil

Every example so far disabled encoding with `allow_unimplemented_encode`.
Turning it on surfaces a shape mismatch: every encoder the derive calls must
match `fn(&T) -> Vec<u8>`, but the built-ins (`encb::u8`, `encb::be_u16`,
`encb::be_u32`, ...) are written as `fn(T) -> Vec<u8>` so they compose on
owned values.

The derive bridges the two shapes with a leading `&` sigil on `enc`:

- `enc = func` - `func` already accepts `&T`, called directly.
- `enc = &func` - `func` accepts owned `T::Borrowed`, routed through `EncodeAs` which
  dereferences via a blanket impl so a `T::Borrowed` feeds a `fn(T)` encoder.

Primitive encoders therefore need the sigil: `enc = &encb::u8`. A hand-written
`Mode::encode_value` already takes `&Mode`, so its field attribute omits the sigil.

For the full dispatch story - how the `&` sigil routes through `EncodeAs`,
which types get zero-cost coercion for free, and when to implement `EncodeAs`
yourself - see [Sigil coercion and `EncodeAs`](../reference/sigil-coercion.md).

## Example

```rust,no_run
{{#include ../../../examples/book_09_encode_sigil.rs}}
```

## Overview

- Derive-called encoders are always `fn(&T) -> Vec<u8>`.
- `enc = &func` is the field-level bridge for owned-argument encoders.
- The sigil is rejected in container `default(...)`; wire per-field encoders instead.

**Next:** [10 - Optional fields & init](./10-init-fallback.md)
