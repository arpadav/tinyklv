# Tutorial 06 - Implementing `DecodeValue`

Tutorial 05 wired a free-standing `decode_celsius` into the field attribute.
That works, but the codec lives *next to* `Celsius`, not *on* it. Any other
KLV struct that wants to embed a `Celsius` has to import the function.

`DecodeValue<S>` lifts the same contract onto the type itself:

```rust
impl DecodeValue<&[u8]> for Celsius {
    fn decode_value(input: &mut &[u8]) -> tinyklv::Result<Self> { ... }
}
```

`Celsius::decode_value` is now a method matching the
`fn(&mut S) -> tinyklv::Result<T>` shape. The field attribute names the
method directly (`dec = Celsius::decode_value`), and every downstream struct
that embeds a `Celsius` gets the decoder for free.

## Example

Run this example: `cargo run --example book_06_decode_value_impl`

```rust,no_run
{{#include ../../../../examples/book_06_decode_value_impl.rs}}
```

## Overview

- `DecodeValue::decode_value` is the trait form of the same `fn(&mut S)` contract.
- A type that implements it can be embedded in any KLV struct without extra wiring.
- Encoding follows the same pattern via `EncodeValue` - covered in Tutorial 09.

**Next:** [07 - Value lengths](./07-val-lengths.md)
