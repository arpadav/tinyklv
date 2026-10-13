# Tutorial 17 - Values with no length

Some formats write a value with no length after its key: a varint ends at its
last byte, a float is always four. Protobuf is one such format. `size(implicit)`
marks such a field, and the derive reads and writes no `len` for it, while the
other fields keep theirs.

- `size(implicit)`: the decoder reads the main stream and stops itself. When it
  runs out of bytes it must leave the cursor at the end of the input, so a value
  split across chunks reads as truncated rather than malformed.
- `size(implicit, exact = N)`: the derive takes `N` bytes for the decoder.

Here `count` is a LEB128 varint and `ratio` four little-endian bytes, both with
no length, and `name` keeps its length prefix. The varint codecs are plain
functions, the shape of [tutorial 05](../fundamentals/05-custom-decoder.md)'s.

Run this example: `cargo run --example book_17_implicit`

```rust
{{#include ../../../../examples/book/advanced/book_17_implicit.rs}}
```

## Overview

- `size(implicit)` drops the length prefix; `exact = N` gives the value its width.
- `default(typ = T, .., size(implicit))` applies it to every `T` field.
- Beside an `implicit` field, a key no field declares is read as
  length-prefixed, unless the struct has [`unknown(..)`](./19-unknown.md).

**Next:** [18 - Framing per key](./18-keylen.md)
