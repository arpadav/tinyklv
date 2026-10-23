# Sigils

**Needs:** [1.1 First packet](../1-basics/01-first-packet.md), [2.2 Codec traits](./02-value-traits.md)

## Default: a reference

The derive hands each field's encoder a reference to the field. Page 2.2's `encode_value` takes `&self`, so it needs no sigil.

## By value: `*`

The built-in number encoders take the value itself. `*` copies it out of the field, so the field's type must be `Copy`:

```rust,ignore
// tinyklv::enc::binary
pub fn be_u16(input: u16, out: &mut Vec<u8>)

#[klv(
    key = 0x02,
    dec = decb::be_u16,
    enc = *encb::be_u16,   // temp, as a u16
)]
temp: u16,
```

## Borrowed view: `&`

The string encoders take a `&str`. `&` hands a field over through its `EncodeAs` impl. For a `String`, that is a `&str`:

```rust,ignore
// tinyklv::enc::string
pub fn from_string_utf8(input: &str, out: &mut Vec<u8>)
```

```diff
 use tinyklv::dec::binary as decb;
+use tinyklv::dec::string as decs;
 use tinyklv::enc::binary as encb;
+use tinyklv::enc::string as encs;
 use tinyklv::prelude::*; // Klv + its traits

     /// Temperature, in 0.01 °C
     temp: u16,
+
+    #[klv(
+        key = 0x03,
+        dec = decs::to_string_utf8,
+        enc = &encs::from_string_utf8, // label, as a &str
+        size(lengthed),                // dec takes the length
+    )]
+    /// Where the sensor is
+    label: String,
 }
```

For `size(lengthed)`, see [Lengthed decoders](../3-sizes/02-lengthed-decoders.md).

```klv
k 03 l 04 v 72 6F 6F 66   // label "roof"
```

## Overview

| sigil | the encoder takes | for example |
|---|---|---|
| none | a reference to the field | `Celsius::encode_value` |
| `*` | a copy of the field | `encb::be_u16` |
| `&` | the field's borrowed view | `encs::from_string_utf8` |

[Sigils and `EncodeAs`](../../reference/sigils.md) lists every borrowed view.

> [!NOTE]
> **A `String` with no sigil.** Remove the `&` from `label`, and it still compiles and writes the same bytes. Rust turns a reference to a string into a `&str` by itself. The `&` matters for a type of your own that Rust cannot convert, by itself, to what its encoder takes. Give that type an `EncodeAs` impl, and the `&` hands its encoder that view.

## Run this example

```sh
cargo run --example book_codecs_sigils
```

```text
Sensor { seq: 42, temp: 2350, label: "roof" } -> [01, 01, 2A, 02, 02, 09, 2E, 03, 04, 72, 6F, 6F, 66]
```

**Try:** remove the `*` from the encoder of `seq`.

<details>
<summary>Answer</summary>

It no longer compiles: without a sigil, `encb::u8` is handed a reference.

```text
error[E0308]: mismatched types
...
 27 |     seq: u8,
    |     ^^^ expected `u8`, found `&u8`
```

</details>

<details>
<summary>Full example</summary>

```rust
{{#include ../../../../examples/book/codecs/book_codecs_sigils.rs}}
```

</details>

**Next:** [Macros](./04-macros.md)
