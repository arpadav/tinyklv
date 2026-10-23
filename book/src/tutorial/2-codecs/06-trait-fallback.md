# Trait fallback

**Needs:** [1.4 Default codecs](../1-basics/04-default-codecs.md), [2.2 Codec traits](./02-value-traits.md)

## Default: the field names its codec

Page 2.2 names `Celsius::decode_value` and `Celsius::encode_value` on the field, though the type already says how it reads and writes.

## Fall back to the traits

With `trait_fallback` on the container, a field with no codec of its own uses its type's traits from page 2.2:

```diff
 #[klv(
     key(
         dec = decb::u8,
         enc = encb::u8,
     ),
     len(
         dec = decb::u8_as_usize,
         enc = encb::u8_from_usize,
     ),
+    trait_fallback,
 )]

-    #[klv(
-        key = 0x02,
-        dec = decb::be_u16,
-        enc = *encb::be_u16,
-    )]
-    /// Temperature, in 0.01 °C
-    temp: u16,
+    #[klv(key = 0x02)]
+    /// Temperature
+    temp: Celsius,
```

`Celsius` and its two impls are page 2.2's.

## Overview

Each of `dec` and `enc` comes from the first of these that has it:

| order | the codec comes from |
|---|---|
| 1 | the field's own `dec` or `enc` |
| 2 | the container's `default(..)` for the field's type |
| 3 | `trait_fallback`: the type's codec traits |

> [!NOTE]
> **An integer field.** The integers, such as `u8` and `u16`, implement neither trait, so the fallback cannot fill them. Give them their own `dec` and `enc`, or a container default. Otherwise the compiler reports that `u8` does not implement `DecodeValue`.
>
> **A lengthed field.** `decode_value` takes no length, so a `lengthed` field needs its own decoder. Without one, the derive stops with a compile error that says so.

## Run this example

```sh
cargo run --example book_codecs_trait_fallback
```

```text
Sensor { seq: 42, temp: Celsius(23.5) } -> [01, 01, 2A, 02, 02, 09, 2E]
```

**Try:** remove `trait_fallback`.

<details>
<summary>Answer</summary>

It no longer compiles. The derive finds no codec for `temp`, and lists the three places one can come from:

```text
error: No encoder is found for the `Celsius` value of field `temp`.
...
       Otherwise, you can:
           a. add a default encoder for all `Celsius` types using `#[klv(default(typ = Celsius, enc = <..>))]` on the struct
           b. add an encoder to `temp` using `#[klv(enc = <..>)]`
           c. set `#[klv(trait_fallback)]` on the struct to fall back to try and use `tinyklv::EncodeValue` trait implementation
```

</details>

<details>
<summary>Full example</summary>

```rust
{{#include ../../../../examples/book/codecs/book_codecs_trait_fallback.rs}}
```

</details>

**Next:** [Nested packets](./07-nested-packets.md)
