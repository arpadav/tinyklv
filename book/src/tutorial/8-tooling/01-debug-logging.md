# Debug logging

**Needs:** [1.1 First packet](../1-basics/01-first-packet.md)

## Default: a result, no steps

`decode_value` returns the struct or an error. When it fails, it does not show the items it read on the way.

## Each item, logged

`debug` logs every key and length the decoder reads, then each field's result:

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
+    debug,
 )]
```

The log goes to stdout, through `println!`. With the `tracing` feature on, it goes to `tracing::debug!` instead. It then shows only through a subscriber you install.

## Run this example

```sh
cargo run --example book_tooling_debug
```

The first line is the example's own; the rest is the log, keys and lengths in decimal:

```text
Sensor { seq: 42, temp: 2350 } -> [01, 01, 2A, 02, 02, 09, 2E]
key: 1, len: 1
	seq: Ok(42)
key: 2, len: 2
	temp: Ok(2350)
```

**Try:** in `wire`, change temp's length from 0x02 to 0x01.

<details>
<summary>Answer</summary>

`unwrap` panics, saying temp is missing from the packet, or its decoder rejected it. The log shows which: temp's length is 1, too short for `be_u16`.

```text
Sensor { seq: 42, temp: 2350 } -> [01, 01, 2A, 02, 02, 09, 2E]
key: 1, len: 1
	seq: Ok(42)
key: 2, len: 1
	temp: Err(ContextError { context: [], cause: None })
```

</details>

<details>
<summary>Full example</summary>

```rust
{{#include ../../../../examples/book/tooling/book_tooling_debug.rs}}
```

</details>

**Next:** [Decode-only and encode-only](./02-one-direction-only.md)
