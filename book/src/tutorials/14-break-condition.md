# 14 - Custom Break Conditions

`#[klv(break)]` and the `BreakCondition` trait let you stop a repeated
decode loop on your own terms: a terminating key, a sentinel-within-a-
sentinel, a CRC mismatch, a count threshold. This chapter walks
through a manual `DecodeValue` implementation that demonstrates the
pattern end-to-end.

## What it teaches

- The `BreakCondition` trait contract (default blanket impl, overriding)
- The `#[klv(break)]` field attribute - when a matching key fires,
  decoding stops
- Skipping unknown keys vs. breaking on them vs. erroring

## The code

```rust,no_run
{{#include ../../../examples/14_break_condition_custom.rs}}
```

## Run it

```sh
cargo run --example 14_break_condition_custom
```

## Key takeaway

Break conditions are the "user hook" in the repeated-decode loop. Use
them for custom termination logic that the declarative attributes
cannot express.

Next: [15 - Tokio End-to-End](./15-tokio-stream-e2e.md).
