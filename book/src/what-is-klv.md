# What is KLV?

**K**ey–**L**ength–**V**alue is a framing pattern: each logical field in a
stream of bytes is prefixed by a key (identifier) and a length (size of the
value in bytes), followed by the value payload. Stacking these triples back
to back gives you a self-describing record:

```text
┌─────┬─────┬───────────────┬─────┬─────┬──────────────┐
│ KEY │ LEN │ VALUE ...     │ KEY │ LEN │ VALUE ...    │
└─────┴─────┴───────────────┴─────┴─────┴──────────────┘
```

Wrap that body in a sentinel (magic bytes) plus a frame-length header and you
have a full KLV packet:

```text
┌──────────┬──────────────┬────────────────────────────┐
│ SENTINEL │ FRAME LENGTH │ KLV TRIPLES ...            │
└──────────┴──────────────┴────────────────────────────┘
```

## Why this pattern?

- **Forward compatibility.** Unknown keys can be skipped - the length tells
  the parser how many bytes to drop. Adding a field does not break old
  decoders.
- **Self-describing.** A decoder does not need to know field offsets up front.
- **Stream-resyncable.** When you lose framing on a noisy transport (UDP,
  radio, RS-422), the sentinel lets you skip garbage and rejoin the stream at
  the next frame boundary.

## Where you see KLV in the wild

- Airborne and ground-sensor telemetry carried inside MPEG-TS video streams.
- Satellite and UAV command-and-control buses.
- Industrial sensor packets on CANbus/RS-485.
- Custom in-house binary protocols that evolved from a "just enough" spec.

## Design choices tinyklv leaves to you

The KLV *shape* is fixed, but the concrete encoding of each part is not:

| Part | Typical choices |
|------|-----------------|
| Sentinel | Any fixed byte string (1–16 bytes is common) |
| Key | 1-byte `u8`, 2-byte `u16`, or multi-byte BER-OID |
| Length | 1-byte `u8`, BER variable-width, or explicit 4-byte `u32` |
| Value | Anything - binary ints, strings, nested KLV, floats |

`tinyklv` provides codec functions for each of these (see
[Codecs reference](../reference/codecs.md)) and lets you mix them on a
per-container and per-field basis.

Move on to the [Hello World tutorial](../tutorials/01-hello-world.md) to see
how these pieces compose.
