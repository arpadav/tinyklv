# Tutorial 15 - Streaming decode with `Decoder<T>`

Chapter 14 showed the manual checkpoint-and-restore loop for feeding a
sync decoder from an async source. `Decoder<T>` is the library version
of that pattern: an owned byte buffer plus a `feed` / `next` interface
that hides the rewind bookkeeping.

`Decoder<T>` only works for sentinel-framed packets. Without a
sentinel there is no way to tell where one packet ends and the next
begins inside a continuous stream - so the derive on your `T` must
carry `sentinel = b"..."`. The decoder scans the buffer for that
sentinel, reads the declared packet length that follows, and only then
runs `DecodePartial::decode_partial` on the exact body.

## The three return values

```rust,ignore
dec.next() // -> Option<Result<T, DecodeError>>
```

| Return | Meaning | What to do |
|--------|---------|------------|
| `Some(Ok(t))` | Full packet decoded | Hand `t` to the application |
| `Some(Err(DecodeError::Malformed(_)))` | Body present but bad | Log; the framed bytes are already dropped so the next packet is not masked |
| `None` | Need more bytes (or no sentinel found yet) | Call `feed` with more bytes |

## Example walkthrough

The runnable example
[`book_15_streaming_decode.rs`](https://github.com/arpadav/tinyklv/blob/main/examples/book_15_streaming_decode.rs)
builds three framed packets, slices the whole byte stream into very
small chunks to simulate a drip-feed transport, and shows that
`Decoder` reassembles every packet without losing progress across
reads.

The core loop is the same four lines you would write for any blocking
byte source:

```rust,ignore
for chunk in incoming_chunks {
    dec.feed(&chunk);
    while let Some(pkt) = dec.next() {
        handle(pkt?);
    }
}
```

No explicit checkpoint, no explicit rewind, no accumulator `Vec<u8>`
owned by the caller. All of that moves into `Decoder<T>`.

## When NOT to use `Decoder<T>`

- **One complete packet at a time**: if each transport unit (e.g. a
  UDP datagram you have already re-assembled) already holds one full
  packet, call `T::decode_frame` or `T::decode_value` directly. The
  extra buffer is wasted.
- **No sentinel**: structs without `sentinel = ...` cannot be framed.
  Use `decode_value` on a bounded slice you built yourself.
- **You need backpressure / async**: `Decoder<T>` is sync. Drive it
  from any runtime by awaiting bytes and feeding them in; the
  decoding itself is CPU-bound.

## Error policy

`Decoder<T>::next()` guarantees forward progress: once it returns
`Some(Err(Malformed))`, the offending framed bytes are drained so the
next call either emits a subsequent packet or asks for more bytes.

If the sentinel genuinely never appears in the stream, the internal
buffer grows without bound until the caller intervenes. Use
`dec.clear()` or a buffer-length check in your feed loop to cap growth
in adversarial scenarios.
