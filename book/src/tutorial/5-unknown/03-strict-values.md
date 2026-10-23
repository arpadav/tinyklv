# Strict decoding

**Needs:** [3.1 Value sub-slices](../3-sizes/01-the-value-is-a-sub-slice.md)

## Default: what the decoder leaves is dropped

Page 3.1's decoder hands `be_u16` only temp's bytes, and drops what it leaves unread. A padded temp decodes:

```klv
k 02             // 1. read the key: temp
l 04             // 2. read the length: 4 bytes follow
v 09 2E 00 00    // 3. be_u16 reads 2350 from the first 2 bytes
                 // 4. the last 2 are unread: dropped
```

A value its decoder rejects is dropped too: temp stays unset, as if key 02 were missing.

## Every value read whole

If your protocol never pads, those bytes mean a broken sender. `strict` makes both cases an error:

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
+    strict,
 )]
```

```klv
k 02             // 1. read the key: temp
l 04             // 2. read the length: 4 bytes follow
v 09 2E 00 00    // 3. be_u16 reads 2350 from the first 2 bytes
                 // 4. the last 2 are unread: ErrorKind::MalformedValue
```

| temp's value | without `strict` | with `strict` |
|---|---|---|
| `be_u16` leaves bytes unread | the bytes are dropped | an error |
| `be_u16` rejects it | the item is dropped | an error |

The encoder does not change: it writes page 1.1's bytes.

> [!NOTE]
> **What `strict` does not check.** It checks each value against its own length, not the input as a whole. An item cut short at the end of the input, such as a lone key byte, is not an error: `decode_value` leaves it in `input`. If the input must hold nothing else, check that `input` is empty after the decode.

## Run this example

```sh
cargo run --example book_unknown_strict
```

```text
Sensor { seq: 42, temp: 2350 } -> [01, 01, 2A, 02, 02, 09, 2E]
invalid `Sensor::temp` value (its decoder left bytes of the item's value unread)
```

**Try:** in `wire`, give temp a length of 1 and one value byte, 09.

<details>
<summary>Answer</summary>

`be_u16` needs 2 bytes and gets 1, so it rejects the value. The second line changes:

```text
invalid `Sensor::temp` value (its decoder rejected the item's value)
```

Both errors are `ErrorKind::MalformedValue`, so the asserts still pass.

</details>

<details>
<summary>Full example</summary>

```rust
{{#include ../../../../examples/book/unknown/book_unknown_strict.rs}}
```

</details>

**Next:** [Break on a key](../6-control/01-stop-on-a-key.md)
