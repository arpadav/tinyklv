# Tokio

**Needs:** [7.2 Streaming decoder](./02-a-live-stream.md)

## A synchronous loop

On page 7.2, a loop feeds the decoder each piece, then drains `iter`.

## An async source

tinyklv has no async API, and needs none: decoding only reads bytes already received. Await each piece, then feed it as before. The decoder needs page 1.3's sentinel:

```diff
 #[derive(Klv, Debug, PartialEq)]
 #[klv(
+    sentinel = b"SENSOR",
     key(
         dec = decb::u8,
         enc = encb::u8,
     ),
```

A sender task sends page 1.3's bytes, 6 at a time, through a tokio channel. The receiver:

```rust,ignore
{{#include ../../../../examples/book/streams/book_streams_tokio.rs:recv}}
```

The loop ends when the sender is dropped, at the end of its task.

## Run this example

```sh
cargo run --example book_streams_tokio
```

```text
recv [DE, AD, 00, 00, 53, 45] -> []
recv [4E, 53, 4F, 52, 07, 01] -> []
recv [01, 2A, 02, 02, 09, 2E] -> [Sensor { seq: 42, temp: 2350 }]
```

**Try:** in the sender task, send 4 bytes at a time.

<details>
<summary>Answer</summary>

Five pieces arrive. The Sensor comes out with the last, which holds temp's last 2 bytes:

```text
recv [DE, AD, 00, 00] -> []
recv [53, 45, 4E, 53] -> []
recv [4F, 52, 07, 01] -> []
recv [01, 2A, 02, 02] -> []
recv [09, 2E] -> [Sensor { seq: 42, temp: 2350 }]
```

</details>

<details>
<summary>Full example</summary>

```rust
{{#include ../../../../examples/book/streams/book_streams_tokio.rs:all}}
```

</details>

**Next:** [Debug logging](../8-tooling/01-debug-logging.md)
