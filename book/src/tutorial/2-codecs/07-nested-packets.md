# Nested packets

**Needs:** [2.2 Codec traits](./02-value-traits.md)

## Default: one value per key

So far, each value is one thing, such as a number or a string, read by one decoder.

## A packet as a value

`#[derive(Klv)]` implements both codec traits of page 2.2 on its struct. So a derived struct can be another struct's field, and its value is a whole KLV body:

```diff
     /// Temperature, in 0.01 °C
     temp: u16,
+
+    #[klv(
+        key = 0x03,
+        dec = Gps::decode_value,
+        enc = Gps::encode_value,
+    )]
+    /// Where the sensor is
+    gps: Gps,
 }
+
+#[derive(Klv, Debug, PartialEq)]
+#[klv(
+    key(
+        dec = decb::u8,
+        enc = encb::u8,
+    ),
+    len(
+        dec = decb::u8_as_usize,
+        enc = encb::u8_from_usize,
+    ),
+)]
+/// A position
+struct Gps {
+    #[klv(
+        key = 0x01,
+        dec = decb::be_i32,
+        enc = *encb::be_i32,
+    )]
+    /// Latitude, in micro-degrees
+    lat: i32,
+
+    #[klv(
+        key = 0x02,
+        dec = decb::be_i32,
+        enc = *encb::be_i32,
+    )]
+    /// Longitude, in micro-degrees
+    lon: i32,
+}
```

With [trait fallback](./06-trait-fallback.md), `gps` needs only its key.

## Byte representation

```klv
k 01 l 01 v 2A                // seq 42
k 02 l 02 v 09 2E             // temp 2350
k 03 l 0C                     // gps: 12 bytes, read by Gps::decode_value
    k 01 l 04 v 03 11 7A D0   // lat 51477200
    k 02 l 04 v FF FE 10 CC   // lon -126772
```

## Run this example

```sh
cargo run --example book_codecs_nested
```

```text
Sensor { seq: 42, temp: 2350, gps: Gps { lat: 51477200, lon: -126772 } } -> [01, 01, 2A, 02, 02, 09, 2E, 03, 0C, 01, 04, 03, 11, 7A, D0, 02, 04, FF, FE, 10, CC]
```

**Try:** give `Gps` lengths of 2 bytes:

```diff
 #[klv(
     key(
         dec = decb::u8,
         enc = encb::u8,
     ),
     len(
-        dec = decb::u8_as_usize,
-        enc = encb::u8_from_usize,
+        dec = decb::be_u16_as_usize,
+        enc = encb::be_u16_from_usize,
     ),
 )]
 /// A position
```

<details>
<summary>Answer</summary>

It prints the new bytes, then decoding the old ones fails. The lengths inside `gps` become 2 bytes, <span class="klv">l 00 04</span>, and every other length stays 1 byte. The outer length counts them: <span class="klv">l 0E</span>, 14 bytes.

```text
Sensor { seq: 42, temp: 2350, gps: Gps { lat: 51477200, lon: -126772 } } -> [01, 01, 2A, 02, 02, 09, 2E, 03, 0E, 01, 00, 04, 03, 11, 7A, D0, 02, 00, 04, FF, FE, 10, CC]
```

</details>

<details>
<summary>Full example</summary>

```rust
{{#include ../../../../examples/book/codecs/book_codecs_nested.rs}}
```

</details>

**Next:** [Value sub-slices](../3-sizes/01-the-value-is-a-sub-slice.md)
