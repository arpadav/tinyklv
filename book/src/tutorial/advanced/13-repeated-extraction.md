# Tutorial 13 - Repeated extraction

Sensor pipelines frequently batch readings back-to-back inside a single UDP
datagram, file record, or log line. When each reading is sentinel-framed,
`drain_frames` peels them off one at a time until the buffer is exhausted.

`tinyklv` exposes this as the `DrainFrames` trait, auto-implemented for any
`T: DecodeFrame<S>` (which requires a sentinel):

```rust,ignore
let readings: Vec<SensorReading> = SensorReading::drain_frames(&mut input)?;
```

`drain_frames` calls `decode_frame` until one fails and returns the packets
before it. It never returns `Err`: input left over is a packet that failed
([Limits](../../reference/limits.md#drain_frames-never-fails)).

For sentinel-less streams, use the `Vec<T>` blanket impl of `DecodeValue`
instead:

```rust,ignore
let readings: Vec<SensorReading> = Vec::<SensorReading>::decode_value(&mut input)?;
```

The example builds five `SensorReading` values, encodes each as a full
frame, concatenates them into one buffer, and drains the whole batch in a
single call.

Run this example: `cargo run --example book_13_repeated_extraction`

```rust,no_run
{{#include ../../../../examples/book/advanced/book_13_repeated_extraction.rs}}
```

- `DrainFrames::drain_frames` returns `Vec<Self>` from a sentinel-framed stream.
- `Vec<T>::decode_value` returns `Vec<T>` from an unframed stream.
- Both stop at the first element that fails and return the ones before it.

**Next:** [14 - Break conditions](./14-break-condition.md)