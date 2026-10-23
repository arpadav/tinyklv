# Exact widths

**Needs:** [1.1 First packet](../1-basics/01-first-packet.md), [3.1 Value sub-slices](./01-the-value-is-a-sub-slice.md)

## Default: any length

The decoder takes as many bytes as the length says, so page 3.1's padded temp decodes:

```klv
k 02             // 1. read the key: temp
l 04             // 2. read the length: 4 bytes follow
v 09 2E 00 00    // 3. be_u16 reads 2350 from the first 2
```

## A fixed width

Key 02 is always a `u16`, 2 bytes. Say so:

```diff
     #[klv(
         key = 0x02,
         dec = decb::be_u16,
         enc = *encb::be_u16,
+        size(exact = 2),
     )]
```

The bytes do not change. What `exact = 2` does:

- encode: writes the length 2 without measuring the value first
- decode: rejects any other length, with `ErrorKind::WrongLength`

```klv
k 02        // 1. read the key: temp, always 2 bytes
l 04        // 2. read the length: 4 is not 2, an error
```

## Run this example

```sh
cargo run --example book_sizes_exact
```

```text
Sensor { seq: 42, temp: 2350 } -> [01, 01, 2A, 02, 02, 09, 2E]
```

**Try:** set `exact = 3`.

<details>
<summary>Answer</summary>

`be_u16` still writes 2 bytes. A debug build checks this, and panics in `encode_value`:

```text
assertion `left == right` failed: tinyklv: the value encoder of `temp` wrote a different byte count than its fixed width
  left: 2
 right: 3
```

A release build does not check. It writes the length `be_u16` produced, 2, and decoding those bytes then fails with `WrongLength`.

</details>

<details>
<summary>Full example</summary>

```rust
{{#include ../../../../examples/book/sizes/book_sizes_exact.rs}}
```

</details>

**Next:** [Reserve hints](./04-reserve-hints.md)
