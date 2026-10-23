# Frames and sentinels

**Needs:** [1.1 First packet](./01-first-packet.md)

## One packet

Page 1.1's bytes hold one packet's items and nothing else: `decode_value` reads them all.

## Packets in a stream

A serial port, a socket or a recorded file carries packets back to back, often with junk between them. The decoder must find where each packet starts and ends.

## Sentinel

A sentinel is fixed bytes that mark the start of every packet:

```diff
 #[derive(Klv, Debug, PartialEq)]
 #[klv(
+    sentinel = b"SENSOR",
     key(
         dec = decb::u8,
         enc = encb::u8,
     ),
     len(
         dec = decb::u8_as_usize,
         enc = encb::u8_from_usize,
     ),
 )]
```

With a sentinel, the struct gains `encode_frame` and `decode_frame`. Call them in place of the value methods:

```diff
     let mut encoded = Vec::new();
-    sensor.encode_value(&mut encoded);
+    sensor.encode_frame(&mut encoded);
     // before the asserts, so an edited value still prints
     println!("{sensor:?} -> {encoded:02X?}");
 
     let mut input = wire.as_slice();
-    let decoded = Sensor::decode_value(&mut input).unwrap();
+    let decoded = Sensor::decode_frame(&mut input).unwrap();
```

`encode_frame` writes the sentinel, the body's length, then the body. `decode_frame` reads it back:

```klv
j DE AD 00 00             // 1. skip bytes up to "SENSOR"
s 53 45 4E 53 4F 52       // 2. match the sentinel
l 07                      // 3. read the body's length with len's decoder: 7 bytes
    k 01 l 01 v 2A        // 4. decode those 7 bytes as on page 1.1
    k 02 l 02 v 09 2E
```

> [!NOTE]
> **More than one frame.** `decode_frame` reads only the first frame, and moves `input` past it. Call it again to read the next one, or see [Draining frames](../7-streams/01-many-frames.md), which reads them all.
>
> **Another input type.** Both decoders read a `&[u8]`. To read another type, set the container's `stream`; see [Container attributes](../../reference/container-attributes.md).
>
> **A length of its own.** The body's length uses `len`'s codec. To give it another, say 2 bytes while each item's length is 1, write `sentinel(value = b"SENSOR", len(dec = .., enc = ..))`; see [Container attributes](../../reference/container-attributes.md#sentinel).

## Run this example

```sh
cargo run --example book_basics_sentinels
```

```text
Sensor { seq: 42, temp: 2350 } -> [53, 45, 4E, 53, 4F, 52, 07, 01, 01, 2A, 02, 02, 09, 2E]
```

**Try:** in `wire`, change the sentinel's last byte from 0x52 to 0x00, then run it again.

<details>
<summary>Answer</summary>

It prints the same line, then `unwrap` panics: with no "SENSOR" in the input, there is no frame to decode.

</details>

<details>
<summary>Full example</summary>

```rust
{{#include ../../../../examples/book/basics/book_basics_sentinels.rs}}
```

</details>

**Next:** [Default codecs](./04-default-codecs.md)
