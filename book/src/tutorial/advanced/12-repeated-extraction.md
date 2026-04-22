# Tutorial 12 - Repeated extraction

Sensor pipelines frequently batch readings back-to-back inside a single UDP
datagram, file record, or log line. Each reading is self-describing - key,
length, value - so they can be peeled off one at a time until the buffer is
exhausted.

`tinyklv` exposes this as the `RepeatedDecode` trait, auto-implemented for any
`T: DecodeValue<S>`:

```rust,ignore
let readings: Vec<SensorReading> = SensorReading::repeated(&mut input)?;
```

Semantics are careful: `repeated` calls `decode_value` until the stream
either cleanly ends (returns whatever was collected so far) or hits a real
parse error (propagates the error with context). Callers are never left
guessing whether an empty `Vec` means "no items" or "parse failed on item
N".

Note that `repeated` is defined on `DecodeValue`, **not** `DecodeFrame` -
the sub-type therefore has no sentinel. Use this pattern for streams of
sentinel-less KLV blobs. If each item is sentinel-framed, loop
`decode_frame` directly instead.

The example builds five `SensorReading` values, concatenates their encoded
value bytes into one buffer, and peels the whole batch off in a single call.

Run this example: `cargo run --example book_12_repeated_extraction`

```rust,no_run
{{#include ../../../../examples/book_12_repeated_extraction.rs}}
```

- `RepeatedDecode::repeated` returns `Vec<Self>` from a sentinel-less stream.
- Clean EOF = success; parse error = propagated error, not silent truncation.
- Available on any type that implements `DecodeValue`, via a blanket impl.

**Next:** [13 - Break conditions](./13-break-condition.md)
