# Latebind

**Needs:** [2.1 Custom decoders](./01-your-own-decoder.md)

## Default: the decoder returns the field

`dec` must return the field's type. To get a `Celsius`, page 2.1 wrote a whole decoder, though `be_u16` already reads the bytes.

## Convert after decoding

`latebind` runs a function on what `dec` returns. Keep `be_u16`, and convert its `u16`:

```diff
     #[klv(
         key = 0x02,
         dec = decb::be_u16,
-        enc = *encb::be_u16,
+        enc = *enc_celsius,
+        latebind = Celsius::from,
     )]
-    /// Temperature, in 0.01 °C
-    temp: u16,
+    /// Temperature
+    temp: Celsius,
 }
+
+#[derive(Debug, PartialEq, Clone, Copy)]
+/// A temperature, in °C
+struct Celsius(f32);
+
+/// From 0.01 °C
+impl From<u16> for Celsius {
+    fn from(centi: u16) -> Self {
+        Celsius(f32::from(centi) / 100.0)
+    }
+}
+
+/// Writes 0.01 °C as a big-endian `u16`
+fn enc_celsius(temp: Celsius, out: &mut Vec<u8>) {
+    encb::be_u16((temp.0 * 100.0).round() as u16, out);
+}
```

```klv
k 02        // 1. read the key: temp
l 02        // 2. read the length: 2 bytes follow
v 09 2E     // 3. be_u16 reads 2350; Celsius::from makes it 23.5
```

`latebind` only runs when decoding, so the encoder still takes a `Celsius`.

## Change in place

With `&mut`, the function changes the decoded value in place, and the field keeps the type `dec` returns. Say the sensor reads at most 125 °C:

```diff
     #[klv(
         key = 0x02,
         dec = decb::be_u16,
         enc = *encb::be_u16,
+        latebind = &mut clamp,
     )]
     /// Temperature, in 0.01 °C
     temp: u16,
 }
+
+/// The sensor reads at most 125 °C
+fn clamp(temp: &mut u16) {
+    *temp = (*temp).min(12_500);
+}
```

```klv
k 02        // 1. read the key: temp
l 02        // 2. read the length: 2 bytes follow
v 32 C8     // 3. be_u16 reads 13000; clamp makes it 12500
```

## Overview

| form | the function | the field's type |
|---|---|---|
| `latebind = f` | takes what `dec` returns, returns the field | what `f` returns |
| `latebind = &mut f` | changes the decoded value in place | the type `dec` returns |

> [!NOTE]
> **A value above the clamp.** The encoder writes the field as it is, unclamped. A temp of 13000 is written as <span class="klv">v 32 C8</span>, and reads back as 12500.

## Run this example

```sh
cargo run --example book_codecs_latebind
```

```text
Sensor { seq: 42, temp: Celsius(23.5) } -> [01, 01, 2A, 02, 02, 09, 2E]
```

```sh
cargo run --example book_codecs_latebind_mut
```

```text
Sensor { seq: 42, temp: 13000 } -> [01, 01, 2A, 02, 02, 32, C8]
[01, 01, 2A, 02, 02, 32, C8] -> Sensor { seq: 42, temp: 12500 }
```

**Try:** remove the `latebind` line.

<details>
<summary>Answer</summary>

It no longer compiles: `be_u16` returns a `u16`, and nothing converts it for the field.

```text
error[E0308]: mismatched types
...
29 |         dec = decb::be_u16,
   |               ^^^^^^^^^^^^ expected `Option<Celsius>`, found `Option<u16>`
```

</details>

<details>
<summary>Full example: convert</summary>

```rust
{{#include ../../../../examples/book/codecs/book_codecs_latebind.rs}}
```

</details>

<details>
<summary>Full example: change in place</summary>

```rust
{{#include ../../../../examples/book/codecs/book_codecs_latebind_mut.rs}}
```

</details>

**Next:** [Trait fallback](./06-trait-fallback.md)
