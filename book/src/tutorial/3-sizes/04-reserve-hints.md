# Reserve hints

**Needs:** [1.1 First packet](../1-basics/01-first-packet.md), [3.2 Lengthed decoders](./02-lengthed-decoders.md)

## Default: what the encoder counts

`encode_value` sets aside room for the whole packet once, before it writes. It counts what it knows: a `u16` is 2 bytes, a key or a length 1 byte. A `String` it cannot count, so the buffer grows, and may move, as the string is written.

## Room for a string

`size(reserve = N)` adds N bytes to that room. It only speeds up encoding: no byte changes, and decoding is untouched. Room for a 16-byte name, on page 3.2's struct:

```diff
     #[klv(
         key = 0x03,
         dec = decs::to_string_utf8,
         enc = &encs::from_string_utf8,
-        size(lengthed),
+        size(lengthed, reserve = 16),
     )]
```

A reserve can also go on the container, for keys or lengths wider than 1 byte:

| where | sets aside |
|---|---|
| a field's `size(reserve = N)` | N bytes for its value |
| `key(.., size(reserve = N))` | N bytes for each key |
| `key(.., size(exact = N))` | the same: N bytes for each key |
| `len(.., size(reserve = N))` | N bytes for each length |

## Run this example

```sh
cargo run --example book_sizes_reserve
```

```text
Sensor { seq: 42, temp: 2350, name: "roof" } -> [01, 01, 2A, 02, 02, 09, 2E, 03, 04, 72, 6F, 6F, 66]
```

**Try:** give temp `size(exact = 2, reserve = 16)`.

<details>
<summary>Answer</summary>

It does not compile: `exact = 2` already tells the encoder the width.

```text
error: conflicting `exact` and `reserve` in `size(..)`
       `exact = N` already reserves its N bytes; remove `reserve`, or remove `exact` if the size is not fixed
```

</details>

<details>
<summary>Full example</summary>

```rust
{{#include ../../../../examples/book/sizes/book_sizes_reserve.rs}}
```

</details>

**Next:** [Values with no length](./05-values-with-no-length.md)
