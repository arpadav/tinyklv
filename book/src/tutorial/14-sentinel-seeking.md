# Tutorial 14 - Sentinel seeking in pipelines

Tutorial 03 used `decode_frame` - one call, one frame. Internally it chains
two steps:

1. **`seek_sentinel`** - advance past arbitrary noise, consume the sentinel
   and the length byte, and return the body region as a sub-stream.
2. **`decode_value`** - parse the body region into the target struct.

Most code wants the combined `decode_frame`. Pipelines that track per-frame
telemetry need step 1 in isolation. Use cases:

- Log the byte offset of every frame as it is located (for tracing, audit,
  or replay).
- Count frames vs. dropped bytes in a noisy capture.
- Keep looping even when `decode_value` fails on a specific frame: because
  `seek_sentinel` already advanced the outer cursor past the sentinel+length,
  a value-decode failure won't desynchronise the next frame.

The idiomatic pipeline looks like:

```rust,ignore
while let Ok(mut body) = HeartbeatPacket::seek_sentinel(&mut buffer) {
    record_frame_offset(/* ... */);
    let packet = HeartbeatPacket::decode_value(&mut body)?;
    /* ... */
}
```

The example concatenates three heartbeats with noise between them, seeks
each one in turn, records its starting offset in the original buffer, and
then decodes the body. The per-frame assertion at the bottom verifies that
the outer cursor and the inner body cursor move independently.

```rust,no_run
{{#include ../../../examples/book_14_sentinel_seeking.rs}}
```

- `seek_sentinel` advances past noise and returns an isolated body sub-stream.
- Errors inside `decode_value` don't bleed back into the outer cursor.
- Use this split when you need per-frame telemetry or offset tracking.

**Next:** [15 - Async / Tokio streams](./15-tokio-streams.md)
