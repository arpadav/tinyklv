# 03 - Strings and Primitives

Real telemetry packets mix fixed-width numbers with variable-length
strings. This example walks through a weather-station registration packet
with a `u32` serial, plus two UTF-8 string fields.

## What it teaches

- `var = true` - the attribute that marks a field as length-parameterised
- The difference between a fixed-width decoder signature
  `fn(&mut &[u8]) -> Result<T>` and a variable-length one
  `fn(len: usize) -> impl Fn(&mut &[u8]) -> Result<T>`
- `tinyklv::dec::binary::to_string_utf8` /
  `tinyklv::enc::string::from_string_utf8`

## The code

```rust,no_run
{{#include ../../../examples/03_strings_and_types.rs}}
```

## Run it

```sh
cargo run --example 03_strings_and_types
```

## Key takeaway

Variable-length codecs are just regular codecs that take the KLV length
as a parameter. `var = true` tells the derive to thread that length
through for you.

Next: [04 - Optional Fields](./04-optional-fields.md).
