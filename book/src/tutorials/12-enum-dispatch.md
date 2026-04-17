# 12 - Enum Dispatch over Streams

Real telemetry buses carry *multiple* packet shapes on the same wire.
To decode them all into a single typed stream, peek at the sentinel (or
a discriminator key), dispatch into the right concrete struct, and wrap
the result in an enum variant.

## What it teaches

- Using two distinct `#[derive(Klv)]` structs with different sentinels
- Peeking the first few bytes before committing to a decoder
- Producing an `enum Packet { ... }` iterator over heterogeneous bytes

## The code

```rust,no_run
{{#include ../../../examples/12_enum_dispatch_stream.rs}}
```

## Run it

```sh
cargo run --example 12_enum_dispatch_stream
```

## Key takeaway

`tinyklv` does not impose a single-container worldview. Dispatch on the
sentinel at the outer layer and your application sees one unified
stream of typed variants.

Next: [13 - Variable-Length Fields](./13-variable-length.md).
