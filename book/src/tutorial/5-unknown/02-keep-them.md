# Keeping unknown items

**Needs:** [2.1 Custom decoders](../2-codecs/01-your-own-decoder.md), [5.1 Skipping or denying unknown keys](./01-skip-or-deny.md)

## Default: skipped and lost

Without `deny_unknown_keys`, page 5.1's decoder skips key 07. Encode the Sensor it returns, and key 07 is gone:

```klv
k 01 l 01 v 2A      // seq  = 42
k 02 l 02 v 09 2E   // temp = 2350
```

A gateway that decodes, edits and re-sends a packet would drop every key it does not know.

## Kept in a field

Two additions keep them:

- `unknown(..)` on the container: the decoder and encoder of every unknown item
- `unknown` on a field: holds the items, as key and value pairs

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
+    unknown(
+        dec = dec_unknown,
+        enc = &enc_unknown,
+    ),
 )]

     /// Temperature, in 0.01 °C
     temp: u16,
+
+    #[klv(unknown)]
+    /// Every item whose key no field declares
+    extra: Vec<(u8, Vec<u8>)>,
 }
+
+/// Reads the rest of an unknown item: its length, then its value
+fn dec_unknown(_key: u8) -> impl Fn(&mut &[u8]) -> tinyklv::Result<Vec<u8>> {
+    |input| {
+        let len = decb::u8_as_usize(input)?;
+        let Some((value, rest)) = input.split_at_checked(len) else {
+            // cut short: consume the rest, so a stream waits for more
+            input.finish();
+            return Err(tinyklv::winnow::error::ParserError::from_input(input));
+        };
+        *input = rest;
+        Ok(value.to_vec())
+    }
+}
+
+/// Writes the rest of a kept item: its length, then its value
+fn enc_unknown(value: &[u8], out: &mut Vec<u8>) {
+    encb::u8_from_usize(value.len(), out);
+    out.extend_from_slice(value);
+}
```

The decoder gets the key and reads the rest of the item:

```klv
k 01 l 01 v 2A      // 1. key 01 selects seq: u8 reads 42
u 07                // 2. key 07 selects no field: call dec_unknown with key 07
u    01 FF          // 3. dec_unknown reads the length, then 1 byte
                    // 4. extra keeps the key and the value: (07, [FF])
k 02 l 02 v 09 2E   // 5. key 02 selects temp: be_u16 reads 2350
```

The encoder writes the fields, then the kept items, in the order read:

```klv
k 01 l 01 v 2A      // 1. write seq
k 02 l 02 v 09 2E   // 2. write temp
u 07 01 FF          // 3. write key 07; enc_unknown writes its length and value
```

> [!NOTE]
> **An item cut short.** If the input ends inside an unknown item, your decoder must move the cursor to the end of the input before it returns the error, as `finish` does above. A streaming decoder then waits for the rest of the item, instead of rejecting the packet.

## Run this example

```sh
cargo run --example book_unknown_keep
```

```text
Sensor { seq: 42, temp: 2350, extra: [(7, [255])] } -> [01, 01, 2A, 02, 02, 09, 2E, 07, 01, FF]
```

**Try:** add a second unknown item to the end of `wire`: key 08, length 2, then AA BB.

<details>
<summary>Answer</summary>

It prints the same line, then its last `assert_eq!` fails: extra holds both items, in order.

```text
assertion `left == right` failed
  left: Sensor { seq: 42, temp: 2350, extra: [(7, [255]), (8, [170, 187])] }
 right: Sensor { seq: 42, temp: 2350, extra: [(7, [255])] }
```

</details>

<details>
<summary>Full example</summary>

```rust
{{#include ../../../../examples/book/unknown/book_unknown_keep.rs}}
```

</details>

**Next:** [Strict decoding](./03-strict-values.md)
