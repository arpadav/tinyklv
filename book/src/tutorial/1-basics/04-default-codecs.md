# Default codecs

**Needs:** [1.1 First packet](./01-first-packet.md)

## A codec on every field

Page 1.1 names a decoder and an encoder on each field. A packet with many fields of one type repeats the same pair on each. Here, temp and a new humidity reading are both a `u16`.

## One codec per type

`default` on the container names the codecs of every field of one type, given by `typ`:

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
+    default(
+        typ = u8,
+        dec = decb::u8,
+        enc = *encb::u8,
+    ),
+    default(
+        typ = u16,
+        dec = decb::be_u16,
+        enc = *encb::be_u16,
+    ),
 )]
```

Each field then needs only its key:

```diff
-    #[klv(
-        key = 0x01,
-        dec = decb::u8,
-        enc = *encb::u8,
-    )]
+    #[klv(key = 0x01)]
     /// Frame counter
     seq: u8,
 
-    #[klv(
-        key = 0x02,
-        dec = decb::be_u16,
-        enc = *encb::be_u16,
-    )]
+    #[klv(key = 0x02)]
     /// Temperature, in 0.01 °C
     temp: u16,
+
+    #[klv(key = 0x03)]
+    /// Relative humidity, in 0.01 %
+    humidity: u16,
```

Temp and humidity now share the one `u16` pair. A field that names its own `dec` or `enc` uses its own.

## Byte representation

```klv
k 01 l 01 v 2A      // seq      = 42
k 02 l 02 v 09 2E   // temp     = 2350
k 03 l 02 v 11 94   // humidity = 4500
```

> [!NOTE]
> **A type written another way.** A default applies to a field whose type is written exactly as its `typ`, or to an `Option` of it. A type alias of `u16` is a different name, so it gets nothing from the `u16` default, and the build fails with "No decoder is found".

## Run this example

```sh
cargo run --example book_basics_default_codecs
```

```text
Sensor { seq: 42, temp: 2350, humidity: 4500 } -> [01, 01, 2A, 02, 02, 09, 2E, 03, 02, 11, 94]
```

**Try:** give temp its own codecs, `le_u16` (little-endian), then run it again.

<details>
<summary>Answer</summary>

It prints temp's bytes swapped, then its first `assert_eq!` fails: temp's own encoder wins over the `u16` default, and humidity keeps it.

```text
Sensor { seq: 42, temp: 2350, humidity: 4500 } -> [01, 01, 2A, 02, 02, 2E, 09, 03, 02, 11, 94]
```

</details>

<details>
<summary>Full example</summary>

```rust
{{#include ../../../../examples/book/basics/book_basics_default_codecs.rs}}
```

</details>

**Next:** [Custom decoders](../2-codecs/01-your-own-decoder.md)
