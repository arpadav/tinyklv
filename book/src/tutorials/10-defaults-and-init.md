# 10 - Defaults and Init

A packet that is missing optional keys should still decode. `tinyklv`
supports two layers of defaults:

- **Container-level** `default(typ = T, dec = path, enc = path)` - a
  codec pair that applies to every field of type `T` unless overridden.
- **Field-level** `default = expr` - a Rust expression that produces the
  fallback value when the key is absent.

## What it teaches

- Declaring a per-type default codec so you don't repeat yourself
- Field-level `default =` expressions and scoped visibility
- How defaults interact with `Option<T>` and `deny_unknown_keys`

## The code

```rust,no_run
{{#include ../../../examples/10_defaults_and_init.rs}}
```

## Run it

```sh
cargo run --example 10_defaults_and_init
```

## Key takeaway

Use container defaults when the same codec pair would otherwise repeat
on every field of a given type. Use field-level `default =` when you
want a concrete non-`None` fallback for a required-shaped field.

Next: [11 - Repeated Extraction](./11-repeated-extraction.md).
