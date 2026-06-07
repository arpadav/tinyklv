# Tutorial 15 - Streaming partial packets

Earlier chapters decoded one bounded input at a time: a full body with
`decode_value`, or a full frame with `decode_frame`. Real transports are
messier. TCP reads are short, ring buffers split arbitrarily, and a
single logical packet may arrive across several chunks.

`tinyklv` models that with **partial packets**. Internally, decoding can
pause and return a partial state containing every field that landed so
far. More bytes can then resume the same packet instead of restarting
from byte zero.

Most callers should not manage that state manually. The usual entry
point is `T::decoder()`, which owns a byte buffer, seeks sentinels, and
drives `decode_partial` for you.

## `::decoder()` for framed byte streams

Use `T::decoder()` when bytes are arriving from a continuous stream and
`T` has a `sentinel = ...`. The decoder solves two problems at once:

- it finds packet boundaries in a noisy byte stream
- it preserves an in-flight partial packet when a body is split across feeds

At the top level, the loop is:

```rust
let mut dec = Heartbeat::decoder();

for chunk in chunks {
    dec.feed(chunk);
    for pkt in dec.iter() {
        handle(pkt);
    }
}
```

That is the main streaming API. `feed()` appends bytes, and `iter()`
drains every packet that is complete right now.

If you already buffered some bytes, `&mut Decoder` also implements
`IntoIterator`, so draining can be written as:

```rust
dec.feed(&buf);
let got: Vec<Heartbeat> = (&mut dec).into_iter().collect();
```

Internally, the decoder does not decode the final type directly. In
fresh mode it:

1. scans for the sentinel
2. reads the declared frame length
3. passes that exact body slice into `DecodePartial::decode_partial`

If the body completes, you get a final `Heartbeat`. If not, the decoder
stores the partial and waits for the next `feed()`. The next decode call
resumes from that partial state instead of re-seeking the sentinel.

This is why `::decoder()` is only for **sentinel-framed streams**.
Without a sentinel there is no way to tell where one top-level packet
ends and the next begins inside a continuous byte stream.

When the upstream reaches EOF, call `finish()` if you want to force the
in-flight partial to finalise:

```rust
let last = dec.finish()?;
```

That succeeds only if every required field has already landed. Otherwise
you get a malformed error naming the missing field.

## `decode_partial` for one packet body at a time

`DecodePartial` is the lower-level contract that powers the decoder.
Unlike `decode_value`, it does not assume the body is complete. Instead
it returns a `Packet<T, P>`:

```rust
match Heartbeat::decode_partial(&mut body) {
    Ok(Packet::Ready(pkt)) => handle(pkt),
    Ok(Packet::NeedMore(partial)) => {
        // save `partial`, append more body bytes, then resume
    }
    Err(label) => report(label),
}
```

The important difference is scope:

- `Decoder` works on a **framed stream** of many top-level packets
- `decode_partial` works on **one packet body** and may pause mid-packet

So direct `decode_partial` is for callers who already know the packet
boundary and want explicit control over the in-flight state. Common
examples are:

- a transport that already separated one frame for you
- a custom buffer manager that stores packet bodies elsewhere
- debugging or testing the exact `NeedMore` / resume behaviour

One subtle point matters here: on `NeedMore`, the cursor is rewound to
the start of the incomplete field. That means the next resume input
should start with the still-unconsumed suffix, then append the new
bytes that just arrived.

Another subtle point is finalisation. `decode_partial` does not treat
EOF as "packet complete" - it treats it as "input ran out." So if *you*
know you have now supplied the entire body, you finalize the returned
partial yourself.

Resuming uses the saved partial:

```rust
let partial = match Heartbeat::decode_partial(&mut first_half)? {
    Packet::NeedMore(p) => p,
    Packet::Ready(pkt) => return Ok(pkt),
};

let mut resumed = Vec::from(first_half);
resumed.extend_from_slice(second_half);
let mut resumed_input = resumed.as_slice();

let pkt = match Heartbeat::resume_partial(&mut resumed_input, partial)? {
    Packet::Ready(pkt) => pkt,
    Packet::NeedMore(partial) => partial.finalize()?,
};
```

The partial type is derive-generated. It mirrors the final struct, but
each field is stored as an `Option<T>`. As bytes arrive, decoded fields
are populated there. Finalisation happens only once decoding is done,
either because the caller does it directly, or because a higher-level
API such as `decode_value` or `Decoder` does it for you.

That is also why `decode_value` behaves differently. It is the one-shot
API, so a `NeedMore` state is treated as truncation and surfaced as an
error. `decode_partial` keeps the partial instead.

## Example

This example shows both levels:

- `Heartbeat::decoder()` on a framed stream split into tiny chunks
- `Heartbeat::decode_partial()` / `resume_partial()` on one split body

Run this example: `cargo run --example book_15_streaming_decode`

```rust
{{#include ../../../../examples/book_15_streaming_decode.rs}}
```

## Overview

- `T::decoder()` is the high-level API for sentinel-framed streams.
- `DecodePartial` is the lower-level API for one packet body that may be incomplete.
- Partial packets preserve already-decoded fields so resume does not restart from byte zero.
- `finish()` is the stream-end hook for forcing the current partial to finalise.

**Next:** [16 - Async / Tokio streams](./16-tokio-streams.md)
