# Optional fields

**Needs:** [1.1 First packet](../1-basics/01-first-packet.md)

## Every field is required

Page 1.1's decoder needs both keys. A packet without temp is an error:

```klv
k 01 l 01 v 2A      // 1. key 01 selects seq: u8 reads 42
                    // 2. the input ends with no key 02: temp is missing, an error
```

## A field the packet may leave out

Say the sensor sends temp only when it has a reading. Make the field an `Option`:

```diff
     #[klv(
         key = 0x02,
         dec = decb::be_u16,
         enc = *encb::be_u16,
     )]
     /// Temperature, in 0.01 °C
-    temp: u16,
+    temp: Option<u16>,
```

The codecs stay `u16`'s: they read and write the value inside the `Option`.

```klv
k 01 l 01 v 2A      // 1. key 01 selects seq: u8 reads 42
                    // 2. the input ends with no key 02: temp is None
```

The encoder mirrors it: `None` writes no item, and `Some` writes page 1.1's item.

> [!NOTE]
> **A temp that fails to decode.** If key 02 is there but its value does not decode, say its length is 1 where `be_u16` needs 2, the decoder drops the item and temp stays `None`, as if key 02 were missing. To make that an error, add `strict` to the container ([5.3 Strict decoding](../5-unknown/03-strict-values.md)).

## Run this example

```sh
cargo run --example book_presence_option
```

```text
Sensor { seq: 42, temp: None } -> [01, 01, 2A]
```

**Try:** set temp to `Some(2350)`, then run it again.

<details>
<summary>Answer</summary>

It prints page 1.1's bytes, then its first `assert_eq!` fails: the expected bytes are still those of `None`.

```text
Sensor { seq: 42, temp: Some(2350) } -> [01, 01, 2A, 02, 02, 09, 2E]
```

</details>

<details>
<summary>Full example</summary>

```rust
{{#include ../../../../examples/book/presence/book_presence_option.rs}}
```

</details>

**Next:** [Default values](./02-fallback-values.md)
