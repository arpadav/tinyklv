# Tutorial 16 - Async / Tokio streams

`tinyklv` has no async surface. `T::decoder()` is synchronous - it takes
`&[u8]` chunks via `feed()` and yields decoded values via `iter()` or
`next()`. That is deliberate: packet framing is CPU-bound, and
wrapping it in an async trait would force every user to pick a runtime
at the library level.

To use it with an async byte source - a UDP socket, a Tokio mpsc
channel, a websocket - you await chunks and hand them to the decoder:

## Pattern 1 - real-time feed / iter

Each `await` may deliver a partial frame. Feed it, drain whatever is
complete, repeat:

```rust,ignore
let mut dec = T::decoder();
while let Some(chunk) = rx.recv().await {
    dec.feed(&chunk);
    for pkt in dec.iter() {
        handle(pkt);
    }
}
```

Every complete packet is emitted immediately after the chunk that
completes it arrives. Partial frames stay in the internal buffer until
the next feed.

## Full example

The example simulates a byte source with a Tokio mpsc channel. The
producer slices each encoded frame into two halves and sends them as
separate chunks, proving the receiver reassembles a frame split across
two awaits.

Run this example: `cargo run --example book_16_tokio_streams`

```rust
{{#include ../../../../examples/book_16_tokio_streams.rs}}
```

## Key takeaways

- `tinyklv` stays sync; async is the caller's concern, one `feed()`
  call away.
- Use `feed()` + `iter()` for real-time packet processing.
- `tokio` appears only as a transport dependency - no library-level
  async feature flag.
