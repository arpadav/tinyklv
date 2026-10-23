# Value sub-slices

**Needs:** [1.1 First packet](../1-basics/01-first-packet.md)

## Default: key, length, value

The decoder reads a key and a length, then hands the field's `dec` only that many bytes, a sub-slice:

```klv
k 02        // 1. read the key: temp
l 02        // 2. read the length: 2 bytes follow
v 09 2E     // 3. be_u16 gets these 2 bytes and reads 2350
```

## A padded value

Say a sender pads temp's value to 4 bytes. The struct stays as it is; only the bytes change:

```diff
     #[rustfmt::skip]
     let wire = [
         0x01, 0x01, 0x2A,       // seq:  key, len, 42
-        0x02, 0x02, 0x09, 0x2E, // temp: key, len, 2350
+        0x02, 0x04,             // temp: key, len 4
+        0x09, 0x2E, 0x00, 0x00, // 2350, then 2 bytes of padding
     ];

     let decoded = Sensor::decode_value(&mut input).unwrap();
-    assert_eq!(encoded, wire);
+    // the encoder writes no padding
+    assert_eq!(encoded, [0x01, 0x01, 0x2A, 0x02, 0x02, 0x09, 0x2E]);
     assert_eq!(decoded, sensor);
```

```klv
k 02             // 1. read the key: temp
l 04             // 2. read the length: 4 bytes follow
v 09 2E 00 00    // 3. be_u16 gets these 4 bytes and reads 2350 from the first 2
```

A decoder never reads past its own value, so a fixed-width decoder still works on a padded one.

> [!NOTE]
> **Bytes a decoder leaves unread.** They are dropped without an error, so the padded bytes above decode as if they had no padding. If your protocol never pads, add `strict` to the container ([5.3 Strict decoding](../5-unknown/03-strict-values.md)): a value whose decoder leaves bytes unread is then an error, `ErrorKind::MalformedValue`.

## Run this example

```sh
cargo run --example book_sizes_subslice
```

```text
Sensor { seq: 42, temp: 2350 } -> [01, 01, 2A, 02, 02, 09, 2E]
```

**Try:** move the two `00` bytes in front of `09 2E`.

<details>
<summary>Answer</summary>

`be_u16` reads the first 2 bytes of its sub-slice, now `00 00`. temp decodes as 0, and the last `assert_eq!` fails:

```text
assertion `left == right` failed
  left: Sensor { seq: 42, temp: 0 }
 right: Sensor { seq: 42, temp: 2350 }
```

</details>

<details>
<summary>Full example</summary>

```rust
{{#include ../../../../examples/book/sizes/book_sizes_subslice.rs}}
```

</details>

**Next:** [Lengthed decoders](./02-lengthed-decoders.md)
