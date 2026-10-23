# Codec traits

**Needs:** [2.1 Custom decoders](./01-your-own-decoder.md)

## Default: functions beside the type

Page 2.1's two functions live beside `Celsius`. Every struct with a `Celsius` field has to import and name both.

## The codec on the type

Implement `DecodeValue` and `EncodeValue` on the type instead, and name its methods:

```diff
     #[klv(
         key = 0x02,
-        dec = decb::be_u16,
-        enc = *encb::be_u16,
+        dec = Celsius::decode_value,
+        enc = Celsius::encode_value,
     )]
-    /// Temperature, in 0.01 °C
-    temp: u16,
+    /// Temperature
+    temp: Celsius,
 }
+
+#[derive(Debug, PartialEq)]
+/// A temperature, in °C
+struct Celsius(f32);
+
+/// Reads 0.01 °C as a big-endian `u16`
+impl DecodeValue<&[u8]> for Celsius {
+    fn decode_value(input: &mut &[u8]) -> tinyklv::Result<Self> {
+        let centi = decb::be_u16(input)?;
+        Ok(Celsius(f32::from(centi) / 100.0))
+    }
+}
+
+/// Writes 0.01 °C as a big-endian `u16`
+impl EncodeValue for Celsius {
+    fn encode_value(&self, out: &mut Vec<u8>) {
+        encb::be_u16((self.0 * 100.0).round() as u16, out);
+    }
+}
```

- `decode_value` has the shape of a decoder from page 2.1
- `&[u8]` is the input it reads, the container's [stream](../1-basics/03-frames-and-sentinels.md)
- `encode_value` takes `&self`, so it needs no sigil: see [Sigils](./03-sigils.md)
- `#[derive(Klv)]` implements both traits on its own struct: see [Nested packets](./07-nested-packets.md)

## Run this example

```sh
cargo run --example book_codecs_value_traits
```

```text
Sensor { seq: 42, temp: Celsius(23.5) } -> [01, 01, 2A, 02, 02, 09, 2E]
```

**Try:** decode a temperature on its own. Add this line to `main`:

```rust,ignore
println!("{:?}", Celsius::decode_value(&mut [0x09, 0x2E].as_slice()));
```

<details>
<summary>Answer</summary>

The method works outside any struct:

```text
Sensor { seq: 42, temp: Celsius(23.5) } -> [01, 01, 2A, 02, 02, 09, 2E]
Ok(Celsius(23.5))
```

</details>

<details>
<summary>Full example</summary>

```rust
{{#include ../../../../examples/book/codecs/book_codecs_value_traits.rs}}
```

</details>

**Next:** [Sigils](./03-sigils.md)
