# Tutorial 15 - Streaming decode with `Decoder<T>`

When bytes arrive in fragments - short TCP reads, ring buffers, UDP
reassembly - a single logical packet may straddle multiple reads.
`Decoder<T>` handles that: it owns a growing byte buffer, scans for the
sentinel, and yields fully-decoded values as enough bytes accumulate.

`Decoder<T>` only works for sentinel-framed packets. Without a
sentinel there is no way to tell where one packet ends and the next
begins inside a continuous stream - so your `T` must carry
`sentinel = b"..."`. The decoder scans the buffer for that sentinel,
reads the declared packet length that follows, and runs
`DecodePartial::decode_partial` on the exact body.

## Three usage patterns

### Pattern 1 - feed + iterate

Feed the whole buffer, then drain with a for-loop. `&mut Decoder`
implements `IntoIterator`, so this works directly:

```rust,ignore
dec.feed(&data);
let got: Vec<T> = (&mut dec).into_iter().collect();
```

### Pattern 2 - chunked feed + `iter()`

Simulate a drip-feed transport: feed small chunks, drain after each
feed. `iter()` returns a borrowing iterator that yields every
currently-complete packet:

```rust,ignore
for chunk in data.chunks(3) {
    dec.feed(chunk);
    for pkt in dec.iter() {
        handle(pkt);
    }
}
```

### Pattern 3 - `consume()`

Hand a chunk iterator directly to the decoder. Internally it feeds
each chunk and yields decoded values as they become complete - the
feed/iter loop in a single expression:

```rust,ignore
let got: Vec<T> = dec.consume(data.chunks(3)).collect();
```

`consume()` accepts anything that implements
`IntoIterator<Item = B>` where `B: AsRef<[u8]>`, so
`Vec<Vec<u8>>`, `&[&[u8]]`, and `slice::chunks()` all work.

## Full example

Run this example: `cargo run --example book_15_a_streaming_decode`

```rust
{{#include ../../../../examples/book_15_a_streaming_decode.rs}}
```

## When NOT to use `Decoder<T>`

- **One complete packet at a time**: if each transport unit already
  holds one full packet, call `T::decode_frame` or `T::decode_value`
  directly. The extra buffer is wasted.
- **No sentinel**: structs without `sentinel = ...` cannot be framed.
  Use `decode_value` on a bounded slice you built yourself.

## Error policy

`Decoder<T>` guarantees forward progress: a malformed framed packet
is drained so the next call either emits a subsequent packet or asks
for more bytes. If the sentinel never appears, the internal buffer
grows without bound - use `dec.clear()` or a buffer-length check in
your feed loop to cap growth.

## `finish()` for stream-end

When the upstream signals "no more bytes coming" (TCP FIN, file EOF),
call `dec.finish()` to force-finalise the in-flight partial. If every
required field has already been populated, you get `Ok(t)`. If not,
you get `Err(DecodeIterError::Malformed(_))` naming the first missing
field. `finish()` consumes the decoder.

> **Next**: Chapter 15 shows how to drive `Decoder<T>` from an async
> byte source with Tokio.

**Next:** [15 - Streaming decode](./15-streaming-decode.md)
