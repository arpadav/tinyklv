# Fixed-width lengths

**Needs:** [1.1 First packet](../1-basics/01-first-packet.md), [3.2 Lengthed decoders](./02-lengthed-decoders.md)

## Default: a length of any width

`len(..)` alone puts no limit on a length, but `u8_from_usize` writes it in 1 byte, which says at most 255. With a 300-byte name in page 3.2's struct, a debug build panics, and a release build writes 44 (300 mod 256).

## Every length is 1 byte

Your protocol's lengths are always 1 byte. Say so, on page 3.2's struct:

```diff
 #[klv(
     key(
         dec = decb::u8,
         enc = encb::u8,
     ),
     len(
         dec = decb::u8_as_usize,
         enc = encb::u8_from_usize,
+        size(exact = 1),
     ),
 )]
```

A name too long for a 1-byte length: a debug build panics, and a release build leaves its item out:

```klv
k 01 l 01 v 2A      // 1. write seq
k 02 l 02 v 09 2E   // 2. write temp
                    // 3. name is 300 bytes, over 255: write no item
```

It logs the skip through `tracing::error!`, or on stderr without the `tracing` feature. A name that fits is written as on page 3.2, only faster.

> [!NOTE]
> **A length encoder of another width.** Under `size(exact = 1)` the length encoder must write exactly 1 byte, as `u8_from_usize` does. If yours writes more for some lengths, a debug build panics when it fills in a string's length. A release build does not check.

## Run this example

```sh
cargo run --release --example book_sizes_len_slot
```

```text
tinyklv: value of 300 bytes does not fit the 1-byte length budget set by `size(exact = 1)`; omitting the item
name of 300 bytes -> [01, 01, 2A, 02, 02, 09, 2E]
```

**Try:** give temp `size(exact = 300)`.

<details>
<summary>Answer</summary>

It does not compile: a 1-byte length cannot say 300.

```text
error: The value of field `temp` is 300 bytes but `len(size(exact = 1))` can encode lengths only up to 255.
```

</details>

<details>
<summary>Full example</summary>

```rust
{{#include ../../../../examples/book/sizes/book_sizes_len_slot.rs}}
```

</details>

**Next:** [Framing per key](./07-framing-per-key.md)
