# Decode-only and encode-only

**Needs:** [1.1 First packet](../1-basics/01-first-packet.md)

## Default: both directions

Every key, length and field names a decoder and an encoder, and the struct gets both `decode_value` and `encode_value`.

## Decode-only

Say a recorder only reads packets, and has no encoders. Leave them out, and the derive stops with an error for each:

```text
error: Missing required `enc` field for `key`.
       If no encoding is required, use `#[klv(allow_unimplemented_encode)]`.
```

`allow_unimplemented_encode` lets them be missing:

```diff
 use tinyklv::dec::binary as decb;
-use tinyklv::enc::binary as encb;
 use tinyklv::prelude::*; // Klv + its traits

 #[derive(Klv, Debug, PartialEq)]
 #[klv(
     key(
         dec = decb::u8,
-        enc = encb::u8,
     ),
     len(
         dec = decb::u8_as_usize,
-        enc = encb::u8_from_usize,
     ),
+    allow_unimplemented_encode,
 )]
 /// One sensor reading
 struct Sensor {
     #[klv(
         key = 0x01,
         dec = decb::u8,
-        enc = *encb::u8,
     )]
     /// Frame counter
     seq: u8,

     #[klv(
         key = 0x02,
         dec = decb::be_u16,
-        enc = *encb::be_u16,
     )]
```

The struct then has no `encode_value` or `encode_frame`. With every encoder still given, it encodes as before.

## Encode-only

Say a sender only writes packets, and has no decoders. Leave them out, and the derive stops with an error for each:

```text
error: Missing required `dec` field for `key`.
       If no decoding is required, use `#[klv(allow_unimplemented_decode)]`.
```

`allow_unimplemented_decode` lets them be missing. Each field's `dec` line goes too:

```diff
-use tinyklv::dec::binary as decb;
 use tinyklv::enc::binary as encb;
 use tinyklv::prelude::*; // Klv + its traits

 #[derive(Klv, Debug, PartialEq)]
 #[klv(
     key(
-        dec = decb::u8,
         enc = encb::u8,
     ),
     len(
-        dec = decb::u8_as_usize,
         enc = encb::u8_from_usize,
     ),
+    allow_unimplemented_decode,
 )]
```

The struct then has no `decode_value` or `decode_frame`.

## Run this example

The decode-only example prints the bytes, then what they decode to:

```sh
cargo run --example book_tooling_one_direction
```

```text
[01, 01, 2A, 02, 02, 09, 2E] -> Sensor { seq: 42, temp: 2350 }
```

```sh
cargo run --example book_tooling_encode_only
```

```text
Sensor { seq: 42, temp: 2350 } -> [01, 01, 2A, 02, 02, 09, 2E]
```

**Try:** in the decode-only example, call `encode_value` on `decoded`.

<details>
<summary>Answer</summary>

It does not compile:

```text
error[E0599]: no method named `encode_value` found for struct `Sensor` in the current scope
```

</details>

<details>
<summary>Full example: decode-only</summary>

```rust
{{#include ../../../../examples/book/tooling/book_tooling_one_direction.rs}}
```

</details>

<details>
<summary>Full example: encode-only</summary>

```rust
{{#include ../../../../examples/book/tooling/book_tooling_encode_only.rs}}
```

</details>

**Next:** [Container attributes](../../reference/container-attributes.md)
