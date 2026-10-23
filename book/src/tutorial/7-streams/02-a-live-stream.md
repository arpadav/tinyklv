# Streaming decoder

**Needs:** [1.3 Frames and sentinels](../1-basics/03-frames-and-sentinels.md), [7.1 Draining frames](./01-many-frames.md)

## All the bytes at once

`decode_frame` and `drain_frames` read a buffer that already holds every frame.

## Bytes that arrive in pieces

A socket or a serial port hands over bytes as they arrive, so one frame can be cut across two reads. With page 1.3's sentinel, the struct gets `decoder`: it makes a decoder that keeps the bytes between reads, and finds each frame:

```diff
 #[derive(Klv, Debug, PartialEq)]
 #[klv(
+    sentinel = b"SENSOR",
     key(
         dec = decb::u8,
         enc = encb::u8,
     ),
```

`feed` it each piece; `iter` then yields every frame that is whole:

```rust,ignore
{{#include ../../../../examples/book/streams/book_streams_decoder.rs:loop}}
```

Page 1.3's bytes, 6 at a time:

```klv
j DE AD 00 00 s 53 45         // 1. feed: no frame yet
s 4E 53 4F 52 l 07 k 01       // 2. feed: the frame is cut, so it waits
l 01 v 2A k 02 l 02 v 09 2E   // 3. feed: the frame is whole: iter yields the Sensor
```

> [!NOTE]
> **When the stream ends.** The decoder yields only whole frames. Bytes still in `buffered` are a frame cut short. `finish` ends the stream: it returns `Ok(None)` when no byte is left. When one is, it returns an error, `ErrorKind::Truncated`.
>
> **A malformed frame.** `iter` stops at it, and drops its bytes. The frames after it come out on the next call to `iter`, which in the loop above comes only with the next piece.

## Run this example

```sh
cargo run --example book_streams_decoder
```

```text
feed [DE, AD, 00, 00, 53, 45] -> []
feed [4E, 53, 4F, 52, 07, 01] -> []
feed [01, 2A, 02, 02, 09, 2E] -> [Sensor { seq: 42, temp: 2350 }]
```

**Try:** delete the last byte of `wire`, then run it again.

<details>
<summary>Answer</summary>

The last feed yields nothing: the frame is cut short, and waits in `buffered`. Then an `assert_eq!` fails.

```text
feed [DE, AD, 00, 00, 53, 45] -> []
feed [4E, 53, 4F, 52, 07, 01] -> []
feed [01, 2A, 02, 02, 09] -> []
```

</details>

<details>
<summary>Full example</summary>

```rust
{{#include ../../../../examples/book/streams/book_streams_decoder.rs:all}}
```

</details>

**Next:** [Partial decoding](./03-one-body-in-pieces.md)
