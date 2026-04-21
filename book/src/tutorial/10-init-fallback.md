# Tutorial 10 - Optional fields & `init`

Protocols evolve. A v1 transmitter emits a four-field heartbeat; a v2
transmitter adds signal strength and battery. If the decoder errors on every
missing key, you cannot read v1 frames with the v2 struct - and the whole
point of KLV is that unknown senders should still produce intelligible bytes.

Two field-level attributes handle the asymmetry:

- **`Option<T>` field** - the derive emits `None` when the key is absent from
  the frame. Use this when the caller genuinely wants "field was missing" to
  be distinguishable from "field was present with some value".
- **`init = expr`** - the derive evaluates `expr` once when the key is absent
  and uses the result as the field value. Use this when the caller wants a
  concrete fallback (a sentinel default, a config-driven constant, a clamp).

Both work in isolation or together. `init` is simply a fallback; the field
type stays `T`, not `Option<T>`.

The example encodes a v1 (four-field) frame with `HeartbeatLegacy`, then
decodes the same bytes as `HeartbeatPacket` (which adds `signal_dbm:
Option<i8>` and `battery_pct: u8` with `init = 100_u8`). The missing keys
produce `None` and `100` respectively - no error, no panic, just a clean
version migration.

```rust,no_run
{{#include ../../../examples/book_10_init.rs}}
```

- `Option<T>` surfaces "key was absent" explicitly to the caller.
- `init = expr` supplies a concrete fallback; the field type stays `T`.
- Both are field-level attributes and compose with every other `#[klv(...)]` option.

**Next:** [11 - Nested packets](./11-nested-packets.md)
