# Lengthed decoders

**Needs:** [1.1 First packet](../1-basics/01-first-packet.md), [2.3 Sigils](../2-codecs/03-sigils.md)

## Default: a fixed width

A field's decoder takes only the input. `be_u16` always reads 2 bytes:

```rust,ignore
// tinyklv::dec::binary
pub fn be_u16(input: &mut &[u8]) -> tinyklv::Result<u16>
```

## A value only its length can size

A string has no fixed width: "roof" is 4 bytes, "attic" is 5. Its decoder takes the length first, and returns the decoder for that many bytes:

```rust,ignore
// tinyklv::dec::string
pub fn to_string_utf8(len: usize) -> impl Fn(&mut &[u8]) -> tinyklv::Result<String>
```

`size(lengthed)` tells the derive to pass it the length. Add a `name`:

```diff
 use tinyklv::dec::binary as decb;
+use tinyklv::dec::string as decs;
 use tinyklv::enc::binary as encb;
+use tinyklv::enc::string as encs;
 use tinyklv::prelude::*; // Klv + its traits

     /// Temperature, in 0.01 °C
     temp: u16,
+
+    #[klv(
+        key = 0x03,
+        dec = decs::to_string_utf8,
+        enc = &encs::from_string_utf8,
+        size(lengthed),
+    )]
+    /// Where the sensor sits
+    name: String,
 }
```

```klv
k 03             // 1. read the key: name
l 04             // 2. read the length: 4 bytes follow
v 72 6F 6F 66    // 3. to_string_utf8(4) reads "roof"
```

`lengthed` changes only the decoder's signature: the bytes are a key, a length and a value, as for any field.

## Run this example

```sh
cargo run --example book_sizes_lengthed
```

```text
Sensor { seq: 42, temp: 2350, name: "roof" } -> [01, 01, 2A, 02, 02, 09, 2E, 03, 04, 72, 6F, 6F, 66]
```

**Try:** remove `size(lengthed)`, then run it again.

<details>
<summary>Answer</summary>

It does not compile. The derive calls `to_string_utf8` as the decoder itself, with the input where it takes the length:

```text
error[E0308]: mismatched types
  --> examples/book/sizes/book_sizes_lengthed.rs:39:15
...
39 |         dec = decs::to_string_utf8,
   |               ^^^^^^^^^^^^^^^^^^^^
   |               |
   |               expected `usize`, found `&mut &[u8]`
```

</details>

<details>
<summary>Full example</summary>

```rust
{{#include ../../../../examples/book/sizes/book_sizes_lengthed.rs}}
```

</details>

**Next:** [Exact widths](./03-exact-widths.md)
