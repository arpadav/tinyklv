# Tutorial 07 - Value lengths

Every field so far had a fixed width. `u8` is one byte, `u16` is two,
`Celsius` is two. The KLV length byte was informational inside the body - the
decoder ignored it because the field's type already told it how many bytes to
read. 

## Subslices

Realistically, what happens is the key is sought, the len value is "grabbed"
as a sub-slice, and only the sub-slice (value) is passed into the decoder. As a result,
you can have functions work which can decode zero-padded values.

In this example, you see that `value: u16` and uses the big-endian `u16` decoder. However,
the length is set to 4. As a result, the slice passed to the decoder will be of 
length 4, but the `u16` decoder will only use the first two values, discarding the
rest. 

Run this example: `cargo run --example book_07_a_subslice`

```rust
{{#include ../../../../examples/book_07_a_subslice.rs}}
```

## Variable length

`String` and other variable length types are different. Its width is only
known at parse time, from the KLV length byte. The codec contract becomes:

```rust
fn(len: usize) -> impl Fn(&mut S) -> Result<T>;
```

which is a function that takes the parsed length and returns a parser specialised to
that length. Setting `varlen = true` on the field tells the derive to call
`decs::to_string_utf8(len)(input)` instead of `decs::to_string_utf8(input)`.

Run this example: `cargo run --example book_07_b_varlen`

```rust
{{#include ../../../../examples/book_07_b_varlen.rs}}
```

## Overview

- `varlen = true` selects the length-taking codec shape.
- `decs::to_string_utf8` is the canonical UTF-8 `String` decoder.
- The outer body length still frames the packet; `varlen` only affects the field.

**Next:** [08 - Latebind transforms](./08-latebind.md)
