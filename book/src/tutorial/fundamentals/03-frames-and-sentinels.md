# Tutorial 03 - Frames & sentinels

`DecodeValue::decode_value` assumed the slice started exactly on a KLV triple. Real
streams are noisier: UDP payloads, log files, and serial ports prepend
preambles, checksums, and other junk. Two new container attributes solve
this - `stream = &[u8]` names the decoder input type (the default, shown for
clarity) and `sentinel = b"HEARTBEAT"` declares the magic bytes that mark the
start of a frame.

With a sentinel configured, the derive implements `DecodeFrame`. Call
`HeartbeatPacket::decode_frame(&mut stream)` and it walks past arbitrary
leading junk until it matches `HEARTBEAT`, reads the length-prefixed body,
and then hands the body to the same `decode_value` you used on Tutorial 01.

## Example

Run this example: `cargo run --example book_03_frame_and_sentinel`

```rust,no_run
{{#include ../../../../examples/book_03_frame_and_sentinel.rs}}
```

## Overview

- `sentinel` adds a `SeekSentinel` implementation. `DecodeFrame` is blanket-impl'd on top.
- `decode_frame` is simply a chain of seeking the sentinel, getting the value, and calling `decode_value`
on the body.

**Next:** [04 - Default codecs](./04-default-codec.md)
