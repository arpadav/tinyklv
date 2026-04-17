# 04 - Optional Fields

Not every field is always present on the wire. `Option<T>` fields are
decoded as `None` when the corresponding key is absent, and skipped on
encode when the value is `None`.

## What it teaches

- Wrapping a field type in `Option<T>` to make its key optional
- How `tinyklv` silently skips missing keys (unless you opt in to
  `deny_unknown_keys`)
- When `None` produces *no* output bytes on encode

## The code

```rust,no_run
{{#include ../../../examples/04_optional_fields.rs}}
```

## Run it

```sh
cargo run --example 04_optional_fields
```

## Key takeaway

`Option<T>` is the idiomatic way to express "this field may or may not
appear in a given packet." Pair it with field-level `default =` (see
chapter 10) when you need a concrete fallback instead of `None`.

Next: [05 - Basic Roundtrip](./05-basic-roundtrip.md).
