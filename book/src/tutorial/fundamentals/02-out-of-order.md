# Tutorial 02 - Out-of-order & prelude

Tutorial 01 imported `Klv` and `DecodeValue` by name. Every subsequent page
replaces those two lines with a single `use tinyklv::prelude::*;` - the
prelude re-exports the derive macro and every trait the book uses.

The more interesting change is the data layout. The `temperature_centideg`
triple now appears *before* the `sequence` triple inside the stream. KLV is keyed,
not positional: fields are located by their `key = ...` attribute, not by offset,
so the decoder happily walks the stream and assembles the struct in whatever
order the triples happened to arrive.

## Example

Run this example: `cargo run --example book_02_out_of_order`

```rust
{{#include ../../../../examples/book_02_out_of_order.rs}}
```

## Overview

- `tinyklv::prelude::*` brings `Klv`, `DecodeValue`, and `DecodeFrame` into scope.
- Reordered triples still decode - the derive dispatches on the key byte.
- The byte layout (inline comments) is the only thing that changed vs 01.

**Next:** [03 - Frames & sentinels](./03-frames-and-sentinels.md)
