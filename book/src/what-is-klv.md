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

And in order to find the start of your KLV packet/frame in a sea of streamed
data, you use a special key called a _sentinel_ (magic bytes), plus a frame-length
header and you have a full KLV packet:

```text
┌────────────────┬─────────────────┬───────────────────┐
│ KEY (SENTINEL) │ LEN (FRAME LEN) │ DATA (KLV's ...)  │
└────────────────┴─────────────────┴───────────────────┘
```

## Why this pattern?

This not only enables nesting, but the sentinel is a fixed byte pattern the decoder scans for to find the next frame boundary. For example:

```text
┌───────────────────────────────────────────────────────┐
│ .. other data ..                                      │
├─────────────────────────┬─────────────────────────────┤
│ 0x03 0x05 .. (SENTINEL) │ 0x13 (PKT LEN = 19 bytes)   │
├─────────────────────────┴─────────────────────────────┤
│ KLV #1 (simple value)                                 │
│ ┌────────┬────────┬─────────────────────────────────┐ │
│ │ K 0x01 │ L 0x02 │ V 0xAA 0xBB                     │ │
│ └────────┴────────┴─────────────────────────────────┘ │
├───────────────────────────────────────────────────────┤
│ KLV #2 (value is a nested KLV train)                  │
│ ┌────────┬────────┬─────────────────────────────────┐ │
│ │ K 0x02 │ L 0x08 │ V (nested KLV, 8 bytes)         │ │
│ └────────┴────────┤                                 │ │
│                   │ ┌────────┬────────┬───────────┐ │ │
│                   │ │ K 0x03 │ L 0x06 │ V 0x04 .. │ │ │
│                   │ └────────┴────────┴───────────┘ │ │
│                   └─────────────────────────────────┘ │
├───────────────────────────────────────────────────────┤
│ KLV #3 (simple value)                                 │
│ ┌────────┬────────┬─────────────────────────────────┐ │
│ │ K 0x05 │ L 0x03 │ V 0xDE 0xAD 0xFF                │ │
│ └────────┴────────┴─────────────────────────────────┘ │
├───────────────────────────────────────────────────────┤
│ .. other data ..                                      │
└───────────────────────────────────────────────────────┘
```

- **Forward compatibility.** Unknown keys can be skipped - the length tells
  the parser how many bytes to drop. Adding a field does not break old
  decoders.
- **Self-describing.** A decoder does not need to know field offsets up front.
- **Stream-resyncable.** When you lose framing on a noisy transport (UDP,
  radio, RS-422), the sentinel lets you skip garbage and rejoin the stream at
  the next frame boundary.

## Design choices `tinyklv` leaves to you

The KLV *shape* is fixed, but the concrete encoding of each part is not:

| Part | Typical choices |
|------|-----------------|
| Sentinel | Any fixed byte string (1–16 bytes is common) |
| Key | 1-byte `u8`, 2-byte `u16`, or multi-byte BER-OID |
| Length | 1-byte `u8`, BER variable-width, or explicit 4-byte `u32` |
| Value | Anything |

`tinyklv` provides codec functions for each of these (see
[Codecs reference](./reference/codecs.md)) and lets you mix them on a
per-container and per-field basis.

Move on to [Tutorial 01 - First packet](./tutorial/fundamentals/01-first-packet.md) to see
how these pieces compose.

## Where you see KLV in the wild

- Airborne and ground-sensor telemetry carried inside MPEG-TS video streams.
- Satellite and UAV command-and-control buses.
- Industrial sensor packets on CANbus/RS-485.
- Custom in-house binary protocols that evolved from a "just enough" spec.

