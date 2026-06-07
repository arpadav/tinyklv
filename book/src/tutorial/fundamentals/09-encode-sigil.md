# Tutorial 09 - Encoding & sigils

## Encode function signature

Every example so far disabled encoding with `allow_unimplemented_encode`. By default, 
`tinyklv` requires both encoding and decoding to be available. Similarly, 
if only encoding/emitting is required, `allow_unimplemented_decode` is also provided.

Similar to decoding, custom encoders can be made. They follow the simple function signature
of:

```rust
fn <name>(input: {1,2,3}, out: &mut Vec<u8>);
```

where the bytes are appended to `out`. The input comes in one of three function signatures,
which use _sigil's_ to describe the format of their inputs. The
derive bridges the shapes with a leading sigil on `enc`:

1. `enc = f`  - `f(&T, &mut Vec<u8>)`
2. `enc = &f` - `f(<T as EncodeAs>::Borrowed, &mut Vec<u8>)`
3. `enc = *f` - `f<T: Copy>(T, &mut Vec<u8>)`

`tinyklv`'s provided encoders, by default, take the literal values themselves, so
most need a sigil, e.g. `enc = &encb::u8`. 

For the full dispatch story - how `&` routes through `EncodeAs`, which
types get zero-cost coercion, and when to implement `EncodeAs`
yourself - see [Sigil coercion and `EncodeAs`](../../reference/sigil-coercion.md).

## Implementing `EncodeValue`

And similar to decoding, `tinyklv::EncodeValue` is used to expose `T::encode_value`, 
which by default takes in `&self`, so it can be used as `#[key(enc = MyStruct::encode_value)]`

## Example 

In this example, a hand-written `Mode::encode_value` already takes `&Mode`, so its
field attribute omits the sigil.

Run this example: `cargo run --example book_09_encode_sigil`

```rust
{{#include ../../../../examples/book_09_encode_sigil.rs}}
```

## Overview

- Derive-called encoders receive `&self.field`; sigils rewrite that.
- `enc = &f` routes through `EncodeAs` (owned containers, smart
  pointers, primitives).
- `enc = *f` is a shortcut for `Copy` primitive types.
- Sigils are accepted on all encoder types, including container-attribute
  `default(...)` blocks as well as on individual fields.

**Next:** [10 - Macros](./10-macros.md)
