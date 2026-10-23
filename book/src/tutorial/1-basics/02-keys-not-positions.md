# Out-of-order keys

**Needs:** [1.1 First packet](./01-first-packet.md)

## In the struct's order

Page 1.1's bytes hold `seq`, then `temp`, in the order the struct declares them:

```klv
k 01 l 01 v 2A      // seq  = 42
k 02 l 02 v 09 2E   // temp = 2350
```

## In any order

Another sender may write the same items in another order. Swap them:

```diff
     let wire = [
-        0x01, 0x01, 0x2A,       // seq:  key, len, 42
         0x02, 0x02, 0x09, 0x2E, // temp: key, len, 2350
+        0x01, 0x01, 0x2A,       // seq:  key, len, 42
     ];
```

The struct does not change. The decoder finds each field by its key, not by its position:

```klv
k 02 l 02 v 09 2E   // 1. key 02 selects temp: be_u16 reads 2350
k 01 l 01 v 2A      // 2. key 01 selects seq: u8 reads 42
                    // 3. the input ends: both fields are set, so it returns the Sensor
```

The encoder always writes the fields in the struct's order, so it still writes page 1.1's bytes.

> [!NOTE]
> **A key seen twice.** If a packet holds the same key twice, the later item wins: the field takes its value, and the earlier one is read and dropped.

## Run this example

```sh
cargo run --example book_basics_keys_not_positions
```

```text
Sensor { seq: 42, temp: 2350 } -> [01, 01, 2A, 02, 02, 09, 2E]
```

**Try:** remove seq's item from `wire`, then run it again.

<details>
<summary>Answer</summary>

It prints the same line, then `unwrap` panics. The error says seq is required: missing from the packet, or rejected by its decoder. To accept a packet without it, see [Optional fields](../4-presence/01-optional-fields.md).

</details>

<details>
<summary>Full example</summary>

```rust
{{#include ../../../../examples/book/basics/book_basics_keys_not_positions.rs}}
```

</details>

**Next:** [Frames and sentinels](./03-frames-and-sentinels.md)
