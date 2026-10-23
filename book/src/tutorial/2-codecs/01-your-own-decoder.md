# Custom decoders

**Needs:** [1.1 First packet](../1-basics/01-first-packet.md)

## Default: a built-in decoder

A decoder is a plain function. It reads bytes off the front of the input and returns the value:

```rust,ignore
// tinyklv::dec::binary
pub fn be_u16(input: &mut &[u8]) -> tinyklv::Result<u16>
```

```klv
k 02        // 1. read the key: temp
l 02        // 2. read the length: 2 bytes follow
v 09 2E     // 3. be_u16 reads 2350
```

## Your own type: Celsius

Say you want `temp` in °C, as a `Celsius`. No built-in returns one, so write a function of the same shape that does, and name it in `dec`. The encoder mirrors it: it takes the value and appends its bytes to `out`.

```diff
     #[klv(
         key = 0x02,
-        dec = decb::be_u16,
-        enc = *encb::be_u16,
+        dec = dec_celsius,
+        enc = *enc_celsius,
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
+/// Reads 0.01 °C as a big-endian `u16`
+fn dec_celsius(input: &mut &[u8]) -> tinyklv::Result<Celsius> {
+    let centi = decb::be_u16(input)?;
+    Ok(Celsius(f32::from(centi) / 100.0))
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
v 09 2E     // 3. dec_celsius: be_u16 reads 2350, divided by 100: 23.5
```

## Run this example

```sh
cargo run --example book_codecs_custom_decoder
```

```text
Sensor { seq: 42, temp: Celsius(23.5) } -> [01, 01, 2A, 02, 02, 09, 2E]
```

**Try:** set temp to 23.456 °C.

<details>
<summary>Answer</summary>

It prints the new bytes, then its first `assert_eq!` fails. `enc_celsius` rounds to the nearest 0.01 °C, so it writes 2346:

```text
Sensor { seq: 42, temp: Celsius(23.456) } -> [01, 01, 2A, 02, 02, 09, 2A]
```

</details>

<details>
<summary>Full example</summary>

```rust
{{#include ../../../../examples/book/codecs/book_codecs_custom_decoder.rs}}
```

</details>

**Next:** [Codec traits](./02-value-traits.md)
