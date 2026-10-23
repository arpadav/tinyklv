# Macros

**Needs:** [2.1 Custom decoders](./01-your-own-decoder.md), [2.3 Sigils](./03-sigils.md)

## Default: a function per field

Page 2.1 wrote `dec_celsius` to read a `u16` and divide it by 100. tinyklv ships that pattern as a macro.

## Scaled values

`scale!` expands to a decoder and `scale_enc!` to its encoder, written in place:

```diff
     #[klv(
         key = 0x02,
-        dec = decb::be_u16,
-        enc = *encb::be_u16,
+        dec = tinyklv::scale!(
+            decb::be_u16,  // reads a u16
+            f32,           // as an f32
+            0.01,          // times 0.01
+        ),
+        enc = tinyklv::scale_enc!(
+            encb::be_u16,  // writes a u16
+            f32,           // from an f32
+            u16,           // cast to u16
+            0.01,          // after dividing by 0.01
+        ),
     )]
-    /// Temperature, in 0.01 °C
-    temp: u16,
+    /// Temperature, in °C
+    temp: f32,
```

```klv
k 02        // 1. read the key: temp
l 02        // 2. read the length: 2 bytes follow
v 09 2E     // 3. be_u16 reads 2350; as an f32, times 0.01: 23.5
```

The encode macros take the field by reference, so they need no sigil.

## Dates and times

With the `chrono` feature, three macros read text of a given length into a chrono date, time or datetime. They only decode, so the field gets its own encoder, `enc_time`:

```diff
+use chrono::NaiveTime;
 use tinyklv::dec::binary as decb;
+use tinyklv::dec::string as decs;
 use tinyklv::enc::binary as encb;
 use tinyklv::prelude::*; // Klv + its traits

     /// Temperature, in 0.01 °C
     temp: u16,
+
+    #[klv(
+        key = 0x03,
+        dec = tinyklv::as_time!(
+            decs::to_string_utf8,  // reads text
+            "%H:%M:%S",            // in this format
+            8,                     // 8 bytes of it
+        ),
+        enc = enc_time,
+    )]
+    /// Time of day of the reading
+    time: NaiveTime,
 }
+
+/// Writes the time as text, the format `as_time!` reads
+fn enc_time(time: &NaiveTime, out: &mut Vec<u8>) {
+    let text = time.format("%H:%M:%S").to_string();
+    out.extend_from_slice(text.as_bytes());
+}
```

```klv
k 03 l 08 v 31 32 3A 33 30 3A 30 30   // time "12:30:00"
```

## Overview

| macro | for | does |
|---|---|---|
| `scale!` | `dec` | reads, casts, multiplies |
| `cast!` | `dec` | reads, casts |
| `scale_enc!` | `enc` | divides, casts, writes |
| `scale_offset_enc!` | `enc` | subtracts an offset, divides, casts, writes |
| `cast_enc!` | `enc` | casts, writes |
| `as_date!` | `dec` | reads text as a `NaiveDate` |
| `as_time!` | `dec` | reads text as a `NaiveTime` |
| `as_datetime!` | `dec` | reads text as a `NaiveDateTime` |

The last three need the `chrono` feature. [Codecs](../../reference/codecs.md) lists each macro's inputs.

## Run this example

```sh
cargo run --example book_codecs_macros
```

```text
Sensor { seq: 42, temp: 23.5 } -> [01, 01, 2A, 02, 02, 09, 2E]
```

```sh
cargo run --features chrono --example book_codecs_macros_chrono
```

```text
Sensor { seq: 42, temp: 2350, time: 12:30:00 } -> [01, 01, 2A, 02, 02, 09, 2E, 03, 08, 31, 32, 3A, 33, 30, 3A, 30, 30]
```

**Try:** set temp to 23.456 °C.

<details>
<summary>Answer</summary>

It prints the new bytes, then its first `assert_eq!` fails. `scale_enc!` casts with `as`, which drops the fraction: it writes 2345. Page 2.1's encoder rounds, to 2346.

```text
Sensor { seq: 42, temp: 23.456 } -> [01, 01, 2A, 02, 02, 09, 29]
```

</details>

<details>
<summary>Full example: scale!</summary>

```rust
{{#include ../../../../examples/book/codecs/book_codecs_macros.rs}}
```

</details>

<details>
<summary>Full example: chrono</summary>

```rust
{{#include ../../../../examples/book/codecs/book_codecs_macros_chrono.rs}}
```

</details>

**Next:** [Latebind](./05-latebind.md)
