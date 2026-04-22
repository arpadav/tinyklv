# Tutorial 09 - Encoding & sigils

Every example so far disabled encoding with `allow_unimplemented_encode`.
Turning it on surfaces a shape mismatch: the derive hands the encoder
`&self.field`, but the built-ins (`encb::u8`, `encb::be_u16`,
`encb::be_u32`, ...) take the value by value so they compose on owned
primitives.

The derive bridges the shapes with a leading sigil on `enc`:

- `enc = func`  - `func` accepts `&T`, called as `func(&self.field)`.
- `enc = &func` - `func` accepts `T::Borrowed`, routed through
  `EncodeAs`, called as `func(EncodeAs::encode_as(&self.field))`.
- `enc = *func` - `func` accepts `T` by value (`T: Copy`), called as
  `func(self.field)`.

Primitive encoders therefore need a sigil: `enc = &encb::u8` works for
any primitive, `enc = *encb::u8` is the shortest path when `T: Copy`.
A hand-written `Mode::encode_value` already takes `&Mode`, so its
field attribute omits the sigil.

For the full dispatch story - how `&` routes through `EncodeAs`, which
types get zero-cost coercion, and when to implement `EncodeAs`
yourself - see [Sigil coercion and `EncodeAs`](../../reference/sigil-coercion.md).

## Example

Run this example: `cargo run --example book_09_encode_sigil`

```rust,no_run
{{#include ../../../../examples/book_09_encode_sigil.rs}}
```

## Overview

- Derive-called encoders receive `&self.field`; sigils rewrite that.
- `enc = &func` routes through `EncodeAs` (owned containers, smart
  pointers, primitives).
- `enc = *func` is a shortcut for `Copy` primitive types.
- Sigils are accepted inside container `default(...)` blocks as well
  as on individual fields.

**Next:** [10 - Optional fields & default](./10-default-fallback.md)
