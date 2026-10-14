# Tutorial 15 - Streaming partial packets

Earlier chapters decoded one bounded input at a time: a full body with
`decode_value`, or a full packet with `decode_frame`. Real transports are
messier. TCP reads are short, ring buffers split arbitrarily, and a
single logical packet may arrive across several chunks.

## `::decoder()` for framed byte streams

Use `T::decoder()` when bytes are arriving from a continuous stream and
`T` has a `sentinel = ...`. It owns a byte buffer, finds packet boundaries in a
noisy stream, and keeps a packet split across feeds until the rest arrives:

```rust
let mut dec = Heartbeat::decoder();

for chunk in chunks {
    dec.feed(chunk);
    for pkt in dec.iter() {
        handle(pkt);
    }
}
```

`feed()` appends bytes, and `iter()` drains every packet that is complete right
now. `&mut Decoder` also implements `IntoIterator`:

```rust
dec.feed(&buf);
let got: Vec<Heartbeat> = (&mut dec).into_iter().collect();
```

For each packet the decoder scans for the sentinel, reads the packet length,
and, once the whole body is buffered, decodes it one-shot with `decode_value`.
Without a sentinel there is no way to tell where one packet ends and the next
begins.

At the end of the stream, a packet left in `buffered()` was cut. `finish()`
finalizes the decoder's partial, which `iter()` never stores, so for a struct
with a required field it is an error.

## `decode_partial` for one packet body at a time

`DecodePartial` works on **one packet body** and may pause inside it, returning
a `Packet<T, P>`:

```rust
match Heartbeat::decode_partial(&mut body) {
    Ok(Packet::Ready(pkt)) => handle(pkt),
    Ok(Packet::NeedMore(partial)) => {
        // save `partial`, append more body bytes, then resume
    }
    Err(error) => report(error),
}
```

It is for callers who already know the packet boundary and want the in-flight
state. On `NeedMore`, the cursor is rewound to the start of the cut item, so the
next input starts with the unconsumed rest, then the new bytes. Running out of
input is never "packet complete" to `decode_partial`, so once you have supplied
the whole body, finalize the partial yourself:

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

The partial type is derive-generated: the struct with each field an
`Option<T>`, filled as items arrive. `decode_value` is the one-shot API: input
that runs out ends its decode, which finalizes what it has.

## Example

The example drives both, the decoder on 3-byte chunks.

Run this example: `cargo run --example book_15_streaming_decode`

```rust
{{#include ../../../../examples/book/advanced/book_15_streaming_decode.rs}}
```

## Overview

- `T::decoder()` is the high-level API for sentinel-framed streams.
- `DecodePartial` is the lower-level API for one packet body that may be incomplete.

**Next:** [16 - Async / Tokio streams](./16-tokio-streams.md)
