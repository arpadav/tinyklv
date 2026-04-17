# 15 - Tokio End-to-End Stream

A full async pipeline: one task encodes KLV frames and hands them over
a `tokio::sync::mpsc` channel; another task seeks the sentinel and
decodes. `tinyklv` itself is synchronous - this chapter shows how the
codec plugs into a real async runtime.

## What it teaches

- Running tinyklv encode/decode inside `tokio::spawn` tasks
- Bridging the sync codec to an async channel with `mpsc`
- Graceful shutdown of a decoder loop on channel close

## The code

```rust,no_run
{{#include ../../../examples/15_tokio_stream_e2e.rs}}
```

## Run it

```sh
cargo run --example 15_tokio_stream_e2e
```

## Key takeaway

tinyklv's codec is sync-by-design - you own the I/O. Wrap it in
whichever async primitive your runtime provides (`mpsc`, `broadcast`,
`UdpSocket`, `TcpStream`) and the codec stays the same.

You've now seen every major feature of `tinyklv`. The
[Reference](../reference/attributes.md) section is the next stop when
you need exact semantics.
