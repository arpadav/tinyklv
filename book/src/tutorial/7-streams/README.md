# Streams

| page | concept | needs |
|---|---|---|
| [7.1 Draining frames](./01-many-frames.md) | `drain_frames`: every frame in a buffer | 1.3 |
| [7.2 Streaming decoder](./02-a-live-stream.md) | `decoder`: frames from bytes that arrive in pieces | 1.3, 7.1 |
| [7.3 Partial decoding](./03-one-body-in-pieces.md) | `decode_partial`: one body in pieces, no sentinel | 1.1 |
| [7.4 Tokio](./04-tokio.md) | the streaming decoder in an async task | 7.2 |
