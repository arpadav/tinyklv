# Tutorial 16 - Async / Tokio streams

`tinyklv` has no async surface. `Decoder<T>` is synchronous - it takes
`&[u8]` chunks via `feed()` and yields decoded values via `iter()` or
`consume()`. That is deliberate: packet framing is CPU-bound, and
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

## Pattern 2 - batch consume

When all chunks are available upfront (or you are happy to buffer
them), collect first, then decode in a single expression:

```rust,ignore
let mut chunks: Vec<Vec<u8>> = Vec::new();
while let Some(chunk) = rx.recv().await {
    chunks.push(chunk);
}
let mut dec = T::decoder();
let decoded: Vec<T> = dec.consume(&chunks).collect();
```

`consume()` accepts `&Vec<Vec<u8>>` because its bound is
`IntoIterator<Item = B>` where `B: AsRef<[u8]>`.

## Full example

The example simulates a byte source with a Tokio mpsc channel. The
producer slices each encoded frame into two halves and sends them as
separate chunks, proving the receiver reassembles a frame split across
two awaits. Both patterns are demonstrated.

Run this example: `cargo run --example book_16_tokio_streams`

```rust
{{#include ../../../../examples/book_16_tokio_streams.rs}}
```

## Key takeaways

- `tinyklv` stays sync; async is the caller's concern, one `feed()`
  call away.
- Use `feed()` + `iter()` for real-time packet processing.
- Use `consume()` when chunks are already collected or come from a
  synchronous source.
- `tokio` appears only as a transport dependency - no library-level
  async feature flag.
