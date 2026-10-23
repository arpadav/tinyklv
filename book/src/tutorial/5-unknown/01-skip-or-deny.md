# Skipping or denying unknown keys

**Needs:** [1.1 First packet](../1-basics/01-first-packet.md)

## Default: skip it by its length

Say a newer sensor also sends key 07, which page 1.1's struct does not declare:

```diff
     let wire = [
         0x01, 0x01, 0x2A,       // seq:  key, len, 42
+        0x07, 0x01, 0xFF,       // key 07, no field: key, len, FF
         0x02, 0x02, 0x09, 0x2E, // temp: key, len, 2350
     ];
```

The decoder skips the item by its length, so an older struct still reads a newer sender's packets:

```klv
k 01 l 01 v 2A      // 1. key 01 selects seq: u8 reads 42
u 07 01 FF          // 2. key 07 selects no field: read its length, skip 1 byte
k 02 l 02 v 09 2E   // 3. key 02 selects temp: be_u16 reads 2350
```

## Deny unknown keys

If your protocol has no other keys, a key 07 means a broken sender. `deny_unknown_keys` makes it an error:

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
+    deny_unknown_keys,
 )]
```

```klv
k 01 l 01 v 2A      // 1. key 01 selects seq: u8 reads 42
u 07                // 2. key 07 selects no field: an error, ErrorKind::UnknownKey
```

The encoder does not change: it writes page 1.1's bytes.

## Run this example

```sh
cargo run --example book_unknown_deny
```

```text
Sensor { seq: 42, temp: 2350 } -> [01, 01, 2A, 02, 02, 09, 2E]
invalid key (expected one of the keys defined on `Sensor`; to turn this off, remove `deny_unknown_keys`)
```

**Try:** remove `deny_unknown_keys`, then run it again.

<details>
<summary>Answer</summary>

It prints the first line, then `unwrap_err` panics: the decoder skips key 07 and returns the Sensor.

```text
called `Result::unwrap_err()` on an `Ok` value: Sensor { seq: 42, temp: 2350 }
```

</details>

<details>
<summary>Full example</summary>

```rust
{{#include ../../../../examples/book/unknown/book_unknown_deny.rs}}
```

</details>

**Next:** [Keeping unknown items](./02-keep-them.md)
