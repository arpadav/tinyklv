# Tutorial 09 - Macros

`tinyklv` ships utility macros (`scale!`, `cast!`, and their encode
counterparts) that expand to closures matching the decoder/encoder
contract. Because they expand to the right signature, they work
**inline** in `dec = ...` and `enc = ...` attributes:

```rust,ignore
const HEADING_SCALE: f64 = 360.0 / 65535.0;

#[klv(
    key = 0x02,
    dec = tinyklv::scale!(decb::be_u16, f64, HEADING_SCALE),
    enc = tinyklv::scale_enc!(
        encb::be_u16, f64, u16, HEADING_SCALE,
    ),
)]
heading_deg: f64,
```

Any macro that expands to the decoder contract works - including your
own. The rule is simple: if it produces
`fn(&mut S) -> tinyklv::Result<T>`, it works in `dec = ...`.

Raw closures (e.g. `dec = |input| { ... }`) do **not** work in
attribute position - that is a proc-macro parsing limitation. Use a
named function instead when the transformation is too complex for
the built-in macros.

## Example

Run the full macro example: `cargo run --example book_10_macro`

```rust
{{#include ../../../../examples/book_10_macro.rs}}
```

See [Reference > Codecs > Utility macros](../../reference/codecs.md)
for the complete macro table with signatures.

## Overview

- Macros allow for expressions and closures to be called in-line, 
  as long as the function signatures match what is expected.

**Next:** [11 - Optional fields, `default`, and fallback impls](./11-default-fallback.md)
