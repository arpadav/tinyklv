# Advanced

Five chapters for production-scale usage: repeated fields, heterogeneous
stream dispatch, variable-width lengths, custom break conditions, and
integration with an async runtime.

- [11 - Repeated Extraction](./11-repeated-extraction.md) - decode N concatenated frames.
- [12 - Enum Dispatch](./12-enum-dispatch.md) - heterogeneous packet streams.
- [13 - Variable-Length Fields](./13-variable-length.md) - BER length integration.
- [14 - Custom Break Conditions](./14-break-condition.md) - early-exit on sentinel data.
- [15 - Tokio End-to-End](./15-tokio-stream-e2e.md) - full async pipeline.
