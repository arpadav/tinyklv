# Tutorial 12 - Repeated extraction

Sensor pipelines frequently batch readings back-to-back inside a single UDP
datagram, file record, or log line. When each reading is sentinel-framed,
`drain_frames` peels them off one at a time until the buffer is exhausted.

`tinyklv` exposes this as the `DrainFrames` trait, auto-implemented for any
`T: DecodeFrame<S>` (which requires a sentinel):

```rust,ignore
let readings: Vec<SensorReading> = SensorReading::drain_frames(&mut input)?;
```

Semantics: `drain_frames` calls `decode_frame` until the stream either
cleanly ends (returns whatever was collected so far) or hits a real parse
error (propagates the error with context). Callers are never left guessing
whether an empty `Vec` means "no items" or "parse failed on item N".

For sentinel-less streams, use the `Vec<T>` blanket impl of `DecodeValue`
instead:

```rust,ignore
let readings: Vec<SensorReading> = Vec::<SensorReading>::decode_value(&mut input)?;
```

The example builds five `SensorReading` values, encodes each as a full
frame, concatenates them into one buffer, and drains the whole batch in a
single call.

Run this example: `cargo run --example book_12_repeated_extraction`

```rust,no_run
{{#include ../../../../examples/book_12_repeated_extraction.rs}}
```

- `DrainFrames::drain_frames` returns `Vec<Self>` from a sentinel-framed stream.
- `Vec<T>::decode_value` returns `Vec<T>` from an unframed stream.
- Clean EOF = success; parse error = propagated error, not silent truncation.

**Next:** [13 - Break conditions](./13-break-condition.md)
