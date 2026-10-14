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

Every complete packet is emitted once the chunk that completes it arrives.

## Full example

The example sends each encoded packet in two halves through a Tokio mpsc
channel, and the receiver reassembles it across two awaits.

Run this example: `cargo run --example book_16_tokio_streams`

```rust
{{#include ../../../../examples/book/advanced/book_16_tokio_streams.rs}}
```

**Next:** [17 - Values with no length](./17-implicit.md)
