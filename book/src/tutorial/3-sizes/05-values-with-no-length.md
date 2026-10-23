# Values with no length

**Needs:** [1.1 First packet](../1-basics/01-first-packet.md), [3.3 Exact widths](./03-exact-widths.md)

## Default: key, length, value

The decoder reads a key, a length, then that many bytes:

```klv
k 02        // 1. read the key: temp
l 02        // 2. read the length: 2 bytes follow
v 09 2E     // 3. take 2 bytes: be_u16 reads 2350
```

## Fixed width, no length

Say key 02 is always a `u16`: its length is always 2. Drop it:

```diff
     #[klv(
         key = 0x02,
         dec = decb::be_u16,
         enc = *encb::be_u16,
+        size(implicit, exact = 2),
     )]
```

```klv
k 02        // 1. read the key: temp, always 2 bytes
v 09 2E     // 2. exact = 2 takes 2 bytes: be_u16 reads 2350
```

| input | encode | decode |
|---|---|---|
| `implicit` | writes no length | reads no length |
| `exact = 2` | `enc` must write 2 bytes | takes 2 bytes for `dec` |

<details>
<summary>Full example: a fixed width</summary>

```rust
{{#include ../../../../examples/book/sizes/book_sizes_implicit_exact.rs}}
```

</details>

## Self-terminating value: BER-OID

A BER-OID ([ITU-T X.690](https://www.itu.int/rec/T-REC-X.690), §8.19) marks its own last byte:

- 7 bits per byte, high bits first
- top bit set: another byte follows

With no fixed width, the decoder reads until it stops:

```diff
+use tinyklv::dec::ber as decber;
 use tinyklv::dec::binary as decb;
+use tinyklv::enc::ber as encber;
 use tinyklv::enc::binary as encb;
 use tinyklv::prelude::*; // Klv + its traits

     #[klv(
         key = 0x02,
-        dec = decb::be_u16,
-        enc = *encb::be_u16,
+        dec = decber::ber_oid,
+        enc = *encber::ber_oid,
+        size(implicit),
     )]
```

```klv
k 02        // 1. read the key: temp
v 92 2E     // 2. ber_oid reads up to the byte whose top bit is 0
```

| byte | top bit | value bits |
|---|---|---|
| <span class="klv">v 92</span> | 1: another byte follows | 0010010 |
| <span class="klv">v 2E</span> | 0: the last byte | 0101110 |

0010010 0101110 in binary is 2350.

## Alternative: LEB128 varint

A LEB128 varint ([protobuf's varint](https://protobuf.dev/programming-guides/encoding/#varints)) works the same, low bits first:

```diff
     #[klv(
         key = 0x02,
-        dec = decb::be_u16,
-        enc = *encb::be_u16,
+        dec = dec_varint,
+        enc = *enc_varint,
+        size(implicit),
     )]
```

```klv
k 02        // 1. read the key: temp
v AE 12     // 2. low 7 bits first: 46, then 18 x 128
```

<details>
<summary>Full example: a varint</summary>

```rust
{{#include ../../../../examples/book/sizes/book_sizes_implicit_varint.rs}}
```

</details>

> [!NOTE]
> If you write your own decoder for an `implicit` value, and the input ends before the value does, move the cursor to the end of the input before returning the error. A streaming decoder then knows the value was cut short and waits for more bytes, instead of rejecting the packet.

## Run this example

```sh
cargo run --example book_sizes_implicit
```

```text
Sensor { seq: 42, temp: 2350 } -> [01, 01, 2A, 02, 92, 2E]
```

**Try:** set temp to 127, then 128.

<details>
<summary>Answer</summary>

It prints the new bytes; the `assert_eq!` then fails, expecting 2350's.

| temp | temp's bytes |
|---|---|
| 127 | <span class="klv">v 7F</span> |
| 128 | <span class="klv">v 81 00</span> |

</details>

<details>
<summary>Full example: BER-OID</summary>

```rust
{{#include ../../../../examples/book/sizes/book_sizes_implicit.rs}}
```

</details>

**Next:** [Fixed-width lengths](./06-length-slot-width.md)
