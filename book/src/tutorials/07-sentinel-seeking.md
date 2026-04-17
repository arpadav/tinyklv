# 07 - Sentinel Seeking

On a noisy transport - UDP drops, radio bursts, bad cabling - you will
occasionally find yourself mid-packet or staring at garbage. The
`SeekSentinel` trait advances the stream to the next occurrence of your
sentinel bytes, giving you an automatic resync.

## What it teaches

- `T::seek_sentinel(&mut s)` - skip arbitrary garbage until the sentinel
  is found or the stream is exhausted
- How `decode_frame` composes seek + length + value decode
- Using `memchr` internally for fast sentinel scans

## The code

```rust,no_run
{{#include ../../../examples/07_sentinel_seeking.rs}}
```

## Run it

```sh
cargo run --example 07_sentinel_seeking
```

## Key takeaway

Pick a sentinel that is unlikely to occur inside your value payloads.
Two to four bytes of high-entropy magic is usually enough. If a payload
*can* legally contain the sentinel, you must escape or length-prefix the
outer frame (and `decode_frame` already does the latter for you).

Next: [08 - Nested Packets](./08-nested-packets.md).
