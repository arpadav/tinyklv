# Break on a key

**Needs:** [1.1 First packet](../1-basics/01-first-packet.md)

## Default: read to the end

`decode_value` reads items until the input ends. It skips a key no field declares, with its length and value.

## An end key, then the next packet

Say each packet ends with key FF, and the next packet follows it at once. The decoder skips FF and reads on, so the next packet's seq replaces this one's:

```klv
k 01 l 01 v 2A      // 1. seq = 42
k 02 l 02 v 09 2E   // 2. temp = 2350
k FF l 00           // 3. key FF: no field has it, so it is skipped
k 01 l 01 v 2B      // 4. the next packet's seq: seq = 43
```

## Break on the key

`break_on` stops the decoder at a key:

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
+    break_on = 0xFF,
 )]
```

```klv
k 01 l 01 v 2A      // 1. seq = 42
k 02 l 02 v 09 2E   // 2. temp = 2350
k FF                // 3. key FF: stop, and return the Sensor
l 00                // 4. left in the input, with the next packet
k 01 l 01 v 2B
```

`break_on` changes only decoding. The encoder writes page 1.1's bytes, with no FF item: no field has that key.

> [!NOTE]
> **The end key's length is left behind.** The decoder stops right after key FF, so the input still starts with FF's length, 00. A second `decode_value` on it reads that 00 as a key, and returns an error. To read past the whole end item, use a [break function](./02-decide-per-key.md).

## Run this example

```sh
cargo run --example book_control_break_on_key
```

```text
Sensor { seq: 42, temp: 2350 } -> [01, 01, 2A, 02, 02, 09, 2E]
```

**Try:** remove the `break_on` line, then run it again.

<details>
<summary>Answer</summary>

It prints the same line, then the second `assert_eq!` fails: the decoded seq is 43, the next packet's.

</details>

<details>
<summary>Full example</summary>

```rust
{{#include ../../../../examples/book/control/book_control_break_on_key.rs}}
```

</details>

**Next:** [Break function](./02-decide-per-key.md)
