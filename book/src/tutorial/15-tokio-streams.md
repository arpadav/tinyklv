# Tutorial 15 - Async / Tokio streams

Tinyklv has no async surface. `decode_frame` is synchronous and takes
`&mut &[u8]`. That is deliberate: packet framing is a CPU-bound string
operation, and wrapping it in a bespoke async trait would force every user
to pick a runtime at the library level.

To use it with an async byte source - a UDP socket, a Tokio mpsc channel, a
websocket - you sit a small buffer-accumulator in front of the sync decoder:

1. Await a chunk from the async source.
2. Append it to a growing `Vec<u8>`.
3. Try `decode_frame` in a loop; each success consumes a complete frame and
   advances a cursor.
4. Drain the consumed bytes from the buffer and wait for the next chunk.

There is one trap. `decode_frame` can partially advance the cursor on a
mid-frame split before failing on the incomplete body - it has already
consumed the sentinel and length by then. The naive version of the loop
would therefore drop the sentinel bytes of the in-progress frame, and the
next chunk would arrive too late to be framed correctly. The fix is to
**checkpoint the cursor before each `decode_frame` attempt and restore it on
failure**, so the buffer keeps the sentinel and length for the retry.

The example simulates a UDP socket with a Tokio mpsc channel. The producer
slices each encoded frame into two halves and sends them as separate chunks,
proving the receiver can reassemble a frame split across two awaits. The
consumer runs the checkpoint-and-restore loop and collects the reassembled
packets; the final `assert_eq!` verifies all three frames round-trip.

```rust,no_run
{{#include ../../../examples/book_15_tokio_streams.rs}}
```

- Tinyklv stays sync; async is the caller's concern, one buffer-accumulator away.
- Save the cursor before each `decode_frame` call and restore it on error.
- `tokio` appears only as a transport dependency - no library-level async feature flag.
