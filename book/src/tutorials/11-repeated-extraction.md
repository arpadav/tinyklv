# 11 - Repeated Extraction

A buffer can hold many concatenated frames. Loop `decode_frame` to
extract them all - sentinel-seeking resyncs after any per-frame error,
so partial corruption in the middle of a stream does not lose the
frames around it.

## What it teaches

- Decoding N frames from a buffer with a `loop { ... }` around
  `decode_frame`
- Stopping cleanly when the input is exhausted
- Recovering from a single bad frame without abandoning the remainder

## The code

```rust,no_run
{{#include ../../../examples/11_repeated_extraction.rs}}
```

## Run it

```sh
cargo run --example 11_repeated_extraction
```

## Key takeaway

`decode_frame` is idempotent in the empty-input case. Loop until it
errors on end-of-stream and you have drained the buffer.

Next: [12 - Enum Dispatch](./12-enum-dispatch.md).
