# One-of fields

**Needs:** [1.4 Default codecs](../1-basics/04-default-codecs.md)

## One key, one field

On page 1.1, key 02 is always temp, in 0.01 °C.

## One field, several keys

Say a sensor sends temp in one of two units, each under its own key:

| key | temp in |
|---|---|
| 02 | 0.01 °C |
| 03 | 0.01 °F |

A packet holds one or the other. Two `Option` fields could both be set; an enum holds exactly one. `oneof` maps each key to a variant:

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
+        typ = u16,
+        dec = decb::be_u16,
+        enc = *encb::be_u16,
+    ),
 )]

     #[klv(
-        key = 0x02,
-        dec = decb::be_u16,
-        enc = *encb::be_u16,
+        key = oneof(
+            0x02 => Temp::Celsius(u16),
+            0x03 => Temp::Fahrenheit(u16),
+        ),
     )]
-    /// Temperature, in 0.01 °C
-    temp: u16,
+    /// Temperature
+    temp: Temp,
 }
+
+#[derive(Debug, PartialEq)]
+/// A temperature, in one of two units
+enum Temp {
+    /// Key 02, in 0.01 °C
+    Celsius(u16),
+    /// Key 03, in 0.01 °F
+    Fahrenheit(u16),
+}
```

A `oneof` field names no codec of its own. Each variant's value uses the container `default` of its type, here the `u16` one. Under `trait_fallback`, its type's traits work too.

```klv
k 03                // 1. read the key: 03 selects temp's Fahrenheit variant
l 02                // 2. read the length: 2 bytes follow
v 1D 06             // 3. u16's default, be_u16, reads 7430: Fahrenheit(7430)
```

The encoder writes the variant that is set, under its own key.

> [!NOTE]
> **Both keys in one packet.** The decoder reads both, and temp keeps the one it read last.
>
> **Neither key.** temp is required, so a packet with neither is an error. Make temp an `Option<Temp>` to allow that: a missing temp is then `None`, which writes no item.

## Run this example

```sh
cargo run --example book_presence_oneof
```

```text
Sensor { seq: 42, temp: Fahrenheit(7430) } -> [01, 01, 2A, 03, 02, 1D, 06]
```

**Try:** set temp to the `Celsius` variant, 2350, then run it again.

<details>
<summary>Answer</summary>

It prints page 1.1's bytes, then its first `assert_eq!` fails: the expected bytes are still those of key 03.

```text
Sensor { seq: 42, temp: Celsius(2350) } -> [01, 01, 2A, 02, 02, 09, 2E]
```

</details>

<details>
<summary>Full example</summary>

```rust
{{#include ../../../../examples/book/presence/book_presence_oneof.rs}}
```

</details>

**Next:** [Skipping or denying unknown keys](../5-unknown/01-skip-or-deny.md)
