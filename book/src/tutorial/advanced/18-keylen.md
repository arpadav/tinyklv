# Tutorial 18 - Framing per key

In tutorial 17 each field's `size(..)` says whether its value has a length.
Some formats put that in the key instead: here, in its low 3 bits. The fused
`keylen(dec, enc)` codec replaces `key(..)` + `len(..)` and frames each value
at run time:

- `dec` reads the key and returns `(key, Framing)`: `Framing::Sliced(n)` when
  the next `n` bytes are the value, which the field decoder gets as a
  sub-slice, or `Framing::Inline` when the field decoder reads the main stream
  and stops itself.
- `enc` gets each key with the length of its encoded value, and writes the
  key, plus the length where the key has one.

The key must decide the framing: a decoder written for a sub-slice reads past
its value when handed the main stream. A `size(lengthed)` field gets its
`Sliced` frame's length.

The struct is tutorial 17's, with no `size(implicit)` anywhere, and encodes to
the same bytes.

Run this example: `cargo run --example book_18_keylen`

```rust
{{#include ../../../../examples/book/advanced/book_18_keylen.rs}}
```

## Overview

- `keylen(dec, enc)` frames each value by its key; the full rules are in
  [Container attributes](../../reference/container-attributes.md#keylendec---enc--).
- A header that fails to read means the rest has not arrived: streaming waits,
  one-shot stops.
- A key no field declares is skipped when `Sliced` and ends the decode when
  `Inline`, unless the struct has [`unknown(..)`](./19-unknown.md).

**Next:** [19 - Unknown frames](./19-unknown.md)
