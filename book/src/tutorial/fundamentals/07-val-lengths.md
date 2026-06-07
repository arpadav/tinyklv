# Tutorial 07 - Value lengths

Every field so far had a fixed width. `u8` is one byte, `u16` is two,
`Celsius` is two. The KLV length byte was informational inside the body - the
decoder ignored it because the field's type already told it how many bytes to
read. 

## Subslices

Realistically, the decoder matches the key, reads the length value, takes the
value as a subslice, and passes only that subslice into the field decoder. As a
result, fixed-width decoders can still decode zero-padded values.

In this example, `value: u16` uses the big-endian `u16` decoder, but the KLV
length is set to 4. The slice passed to the decoder is 4 bytes long, but the
`u16` decoder uses only the first two bytes and leaves the padding behind. 

Run this example: `cargo run --example book_07_a_subslice`

```rust
{{#include ../../../../examples/book_07_a_subslice.rs}}
```

## Variable length

`String` and other variable-length types are different. Their width is only
known at parse time, from the KLV length value. The codec contract becomes:

```rust
fn(len: usize) -> impl Fn(&mut S) -> Result<T>;
```

which is a function that takes the parsed length and returns a parser specialised to
that length. Setting `size(var)` on the field tells the derive to call
`decs::to_string_utf8(len)(input)` instead of `decs::to_string_utf8(input)`.

Run this example: `cargo run --example book_07_b_size_var`

```rust
{{#include ../../../../examples/book_07_b_size_var.rs}}
```

## Overview

- `size(var)` selects the length-taking codec shape.
- `decs::to_string_utf8` is the canonical UTF-8 `String` decoder.
- The outer body length still frames the packet; `size(var)` only affects the field value decoder.

**Next:** [08 - Latebind transforms](./08-latebind.md)
