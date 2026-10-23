# Framing per key

**Needs:** [1.1 First packet](../1-basics/01-first-packet.md), [3.5 Values with no length](./05-values-with-no-length.md)

## Default: a key, then a length

`key(..)` reads every key, `len(..)` every length. Only the struct can drop a length, with `size(implicit)`.

## The key says if a length follows

Say the key's top bit says whether a length byte follows. `keylen(..)` reads a key and its length with one codec, so the key decides:

```diff
 #[klv(
-    key(
-        dec = decb::u8,
-        enc = encb::u8,
-    ),
-    len(
-        dec = decb::u8_as_usize,
-        enc = encb::u8_from_usize,
-    ),
+    keylen(
+        dec = dec_header,
+        enc = enc_header,
+    ),
 )]

     #[klv(
-        key = 0x02,
+        key = 0x82, // top bit set: a length follows
         dec = decb::be_u16,
         enc = *encb::be_u16,
     )]
```

`dec_header` returns the key and its `Framing`:

```rust
{{#include ../../../../examples/book/sizes/book_sizes_keylen.rs:header}}
```

| `Framing` | the field's decoder gets |
|---|---|
| `Sliced(n)` | the next n bytes, as a sub-slice |
| `Inline` | the bytes that follow; it must stop itself |

```klv
k 01        // 1. top bit 0: Inline
v 2A        // 2. u8 reads 1 byte, and stops
k 82        // 3. top bit 1: dec_header reads the length
l 02        // 4. Sliced(2): the next 2 bytes go to be_u16
v 09 2E     // 5. be_u16 reads 2350
```

## Alternative: a wire type in the low 3 bits

Or the key's low 3 bits name a framing, a wire type, as in [protobuf](https://protobuf.dev/programming-guides/encoding/#structure):

| wire type | the value | `Framing` |
|---|---|---|
| 0 | a varint, which stops itself | `Inline` |
| 2 | a length byte, then that many bytes | `Sliced(len)` |

The same `keylen(..)`, with page 3.5's varint for temp:

```diff
     #[klv(
-        key = 0x01,
+        key = 0x0A, // field 1, wire type 2
         dec = decb::u8,
         enc = *encb::u8,
     )]

     #[klv(
-        key = 0x02,
-        dec = decb::be_u16,
-        enc = *encb::be_u16,
+        key = 0x10, // field 2, wire type 0
+        dec = dec_varint,
+        enc = *enc_varint,
     )]
```

```rust
{{#include ../../../../examples/book/sizes/book_sizes_keylen_wiretype.rs:header}}
```

```klv
k 0A        // 1. wire type 2: dec_header reads the length
l 01        // 2. Sliced(1): the next byte goes to u8
v 2A        // 3. u8 reads 42
k 10        // 4. wire type 0: Inline
v AE 12     // 5. dec_varint reads 2350, and stops
```

<details>
<summary>Full example: a wire type</summary>

```rust
{{#include ../../../../examples/book/sizes/book_sizes_keylen_wiretype.rs:all}}
```

</details>

> [!NOTE]
> **One framing per key.** The decoder for a value is picked by its key alone. Return the same framing for a key every time: a decoder that reads to the end of its input reads past its own value when its key comes `Inline`.
>
> **A key no field declares.** It is skipped when `Sliced`. When `Inline`, there is no length to skip it by, so decoding stops before it.
>
> **What cannot sit beside `keylen`.** Each of these is a compile error:
>
> - `key(..)` or `len(..)`: its one codec replaces both
> - a `sentinel` without its own `len` ([Container attributes](../../reference/container-attributes.md#sentinel))
> - a field's `size(implicit)`: the key already says whether a length follows

## Run this example

```sh
cargo run --example book_sizes_keylen
```

```text
Sensor { seq: 42, temp: 2350 } -> [01, 2A, 82, 02, 09, 2E]
```

**Try:** set seq's key to 0x81.

<details>
<summary>Answer</summary>

A length byte now follows seq's key:

```text
Sensor { seq: 42, temp: 2350 } -> [81, 01, 2A, 82, 02, 09, 2E]
```

Decoding the old bytes then fails: key 01 names no field and has no length to skip, so decoding stops there, with seq missing.

</details>

<details>
<summary>Full example: the key's top bit</summary>

```rust
{{#include ../../../../examples/book/sizes/book_sizes_keylen.rs:all}}
```

</details>

**Next:** [Optional fields](../4-presence/01-optional-fields.md)
