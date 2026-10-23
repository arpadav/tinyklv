# Sigils and `EncodeAs`

A sigil before an encoder says how the field is handed to it: by reference, as a copy, or as a borrowed view. [2.3 Sigils](../tutorial/2-codecs/03-sigils.md) teaches it.

## Overview

```rust,ignore
enc = f,     // fn(&T, &mut Vec<u8>)
enc = *f,    // fn(T, &mut Vec<u8>)
enc = &f,    // fn(T::Borrowed<'_>, &mut Vec<u8>)
```

| sigil | the encoder is handed | use it for |
|---|---|---|
| none | a reference to the field | an encoder that takes a reference, such as `encode_value` |
| `*` | a copy of the field, which must be `Copy` | the built-in [binary encoders](./codecs.md#integers-and-floats) |
| `&` | the field's `EncodeAs` view | an encoder of a borrowed form, such as `&str` for a `String` |

A sigil goes on:

- a field's `enc`
- the `enc` of a container's `default(..)`
- the `enc` of a container's `unknown(..)`

The key and length encoders take none: they already receive the key or length by value, and a sigil there is a compile error.

## Example

One field per sigil:

```rust,ignore
#[klv(
    key = 0x01,
    dec = decb::u8,
    enc = *encb::u8,                 // a copy of seq
)]
seq: u8,

#[klv(
    key = 0x02,
    dec = decb::be_u16,
    enc = enc_temp,                  // fn(&u16, &mut Vec<u8>)
)]
temp: u16,

#[klv(
    key = 0x03,
    dec = decs::to_string_utf8,
    enc = &encs::from_string_utf8,   // label's EncodeAs view: a &str
    size(lengthed),
    latebind = Label,                // the decoded String, as a Label
)]
label: Label,
```

```klv
k 01 l 01 v 2A             // seq   = 42
k 02 l 02 v 09 2E          // temp  = 2350
k 03 l 04 v 72 6F 6F 66    // label = "roof"
```

## `EncodeAs`

```rust,ignore
pub trait EncodeAs {
    type Borrowed<'a> where Self: 'a;
    fn encode_as(&self) -> Self::Borrowed<'_>;
}
```

The `&` sigil hands the encoder what `encode_as` returns. tinyklv implements it for:

| type | `Borrowed` |
|---|---|
| the integers, the floats, `bool` and `char` | a reference to itself |
| `String` | `&str` |
| `Cow<str>` | `&str` |
| `Vec<T>` | `&[T]` |
| `Cow<[T]>` | `&[T]` |
| `Box<T>` | `&T` |
| `Rc<T>` | `&T` |
| `Arc<T>` | `&T` |

### Your own type

Implement `EncodeAs` for a type whose encoder takes a borrowed form other than a reference to itself. Here, the `Label` of the example above, written as its text:

```rust,ignore
{{#include ../../../examples/book/ref/book_ref_sigils.rs:label}}
```

> [!NOTE]
> **A `String` with no sigil.** It still compiles, and writes the same bytes: Rust turns a reference to a `String` into a `&str` by itself. Rust does not do that for a type of your own: without the `&`, the `Label` field does not compile.
>
> **A type whose encoder takes a reference to it.** It needs no `EncodeAs`: write no sigil, as for any `encode_value`.

## Run this example

```sh
cargo run --example book_ref_sigils
```

<details>
<summary>Full example: one field per sigil, and EncodeAs</summary>

```rust
{{#include ../../../examples/book/ref/book_ref_sigils.rs:all}}
```

</details>
