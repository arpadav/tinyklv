# Tutorial 07 - Value lengths

Every field so far had a fixed wire width. `u8` is one byte, `u16` is two,
`Celsius` is two. The KLV length byte was informational inside the body - the
decoder ignored it because the field's type already told it how many bytes to
read. 

## Subslices

Realistically, what happens is the key is sought, the len value is "grabbed"
as a sub-slice, and only the sub-slice (value) is passed into the decoder. As a result,
you can have functions work like the following:

Run this example: `cargo run --example book_07_a_subslice`

```rust,no_run
{{#include ../../../../examples/book_07_a_subslice.rs}}
```

## Variable length

`String` is different. Its wire width is only known at parse time, from the
KLV length byte. The codec contract shifts from
`fn(&mut S) -> Result<T>` to `fn(len: usize) -> impl Fn(&mut S) -> Result<T>`:
a function that takes the parsed length and returns a parser specialised to
that length. Setting `varlen = true` on the field tells the derive to call
`decs::to_string_utf8(len)(input)` instead of `decs::to_string_utf8(input)`.

Run this example: `cargo run --example book_07_b_varlen`

```rust,no_run
{{#include ../../../../examples/book_07_b_varlen.rs}}
```

## Overview

- `varlen = true` selects the length-taking codec shape.
- `decs::to_string_utf8` is the canonical UTF-8 `String` decoder.
- The outer body length still frames the packet; `varlen` only affects the field.

**Next:** [08 - Latebind transforms](./08-latebind.md)
