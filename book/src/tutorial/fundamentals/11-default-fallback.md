# Tutorial 11 - Optional fields, `default`, and fallback impls

### `Option<T>` fields

Optional types are handled with ease. During encoding, they are omitted when `None`
and encoded as expected when `Some`. During decoding, they simply default to `None`,
but when encountered, it will populate it as `Some`.

This can be useful in majority of cases. 

### `default` / `default = expr` field attributes

***NOTE:*** _these are different than the `default` container attributes, which define
default decoders/encoders_

The `default` _field_ attribute is similar to a `serde(default)`, where values
are initialized using `<T as Default>::default()`. 

Rather, you can set a default value using `default = ..`, if you want it initialized
using a different value than `<T as Default>::default`

For example

```rust
#[derive(Default)]
enum Color {
    #[default]
    Red,
    Green,
    Blue,
}

#[derive(Klv)]
#[klv(..)]
struct Paint {
    #[klv(default)]
    color_first: Color,     // <-- initialized using Color::Red
    
    #[klv(default = Color::Blue)]
    color_second: Color     // <-- initialized using Color::Blue
}
```

A short packet with three keys: a required `sequence`, an optional
`signal_dbm`, and a `battery_pct` that falls back to `100` when absent.
The thin stream below is hand-rolled to show the grammar - just
`key, len, value` triples. Decoding it produces `None` and `100` for the
two missing keys; no error, no panic.

Run this example: `cargo run --example book_11_a_default`

```rust
{{#include ../../../../examples/book_11_a_default.rs}}
```

## Fallback Implementation

### `trait_fallback` (container-level)

Any field lacking a `enc`/`dec` in any location will automatically cause
a proc-macro error. However, more frequently than not, `tinyklv::EncodeValue`
and `tinyklv::DecodeValue` traits are utilized for impls on custom types.

As a result, it makes sense to use these instead of writing them out explicitly
everytime. Therefore, we can use the `trait_fallback` container attribute
to use the traits as a final fallback for finding the decoder/encoder:

```rust
#[derive(Klv)]
#[klv(..)]
struct Paint {
    #[klv(
        key = 0x01,
        dec = Size::decode_value,
        enc = Size::encode_value,
    )]
    size: Size,
    
    #[klv(
        key = 0x02,
        dec = Color::decode_value,
        enc = Color::encode_value,
    )]
    color: Color,
}
```

Instead, you can write:

```rust
#[derive(Klv)]
#[klv(
    ..,
    trait_fallback,
)]
struct Paint {
    #[klv(key = 0x01)]
    size: Size,
    
    #[klv(key = 0x02)]
    color: Color,
}
```

Where `Size` and `Color` in both examples implement `tinyklv::EncodeValue` and
`tinyklv::DecodeValue`

## Examples

Run this example: `cargo run --example book_11_b_fallback`

```rust
{{#include ../../../../examples/book_11_b_fallback.rs}}
```

## Overview

- `Option<T>` composes: a missing key stays `None`, a present one defers
  to `T`'s impls.
- Bare field `default` (no `=`) calls `Default::default()` on the field type -
  handy when the type already has a sensible zero value.
- Field `default = expr` supplies a concrete fallback; the field type stays `T`.
- Field-level `enc` / `dec` take first precedence in which codec to use.
- Container `default(typ = T, ..)` take second precedence in which codec to use.
- Fallback runs last - it is opt-in and silent unless reached.

**Next:** [12 - Nested packets](../advanced/12-nested-packets.md)
