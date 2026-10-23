# Draining frames

**Needs:** [1.3 Frames and sentinels](../1-basics/03-frames-and-sentinels.md)

## One frame

`decode_frame` reads one frame, and moves the input past it.

## Frames back to back

A recorded file holds frames back to back. `drain_frames` calls `decode_frame` until it fails. It returns every frame before that, in a `Vec`. It needs page 1.3's sentinel:

```diff
 #[derive(Klv, Debug, PartialEq)]
 #[klv(
+    sentinel = b"SENSOR",
     key(
         dec = decb::u8,
         enc = encb::u8,
     ),
```

```diff
     let mut input = wire.as_slice();
-    let decoded = Sensor::decode_value(&mut input).unwrap();
+    let decoded = Sensor::drain_frames(&mut input).unwrap();
```

```klv
j DE AD 00 00            // 1. skip bytes up to "SENSOR"
s 53 45 4E 53 4F 52      // 2. the first frame: seq 42
l 07
    k 01 l 01 v 2A
    k 02 l 02 v 09 2E
s 53 45 4E 53 4F 52      // 3. the second frame: seq 43
l 07
    k 01 l 01 v 2B
    k 02 l 02 v 09 2F
                         // 4. no frame left: return both
```

> [!NOTE]
> **A frame cut short.** `drain_frames` stops at the first frame it cannot decode, and returns the frames before it. It never returns an error. The input is left just after the last good frame, so you can keep the rest until more bytes arrive, or use the [streaming decoder](./02-a-live-stream.md), which does this for you.
>
> **Without a sentinel.** A `Vec<Sensor>` decodes too, but each Sensor reads items until the input ends: two packets back to back decode as one Sensor, with the second one's values. Use it only when each packet ends itself, as with a [break function](../6-control/02-decide-per-key.md).

## Run this example

```sh
cargo run --example book_streams_drain_frames
```

```text
Sensor { seq: 42, temp: 2350 }
Sensor { seq: 43, temp: 2351 }
```

**Try:** delete the last byte of `wire`, then run it again.

<details>
<summary>Answer</summary>

It prints only the first Sensor, then an `assert_eq!` fails. The second frame is cut short, so `drain_frames` stops before it.

```text
Sensor { seq: 42, temp: 2350 }
```

</details>

<details>
<summary>Full example</summary>

```rust
{{#include ../../../../examples/book/streams/book_streams_drain_frames.rs}}
```

</details>

**Next:** [Streaming decoder](./02-a-live-stream.md)
