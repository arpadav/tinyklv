# Summary

[Introduction](./intro.md)
[What is KLV?](./what-is-klv.md)

# Tutorial

- [Fundamentals](./tutorial/fundamentals/README.md)
    - [First packet](./tutorial/fundamentals/01-first-packet.md)
    - [Out-of-order & prelude](./tutorial/fundamentals/02-out-of-order.md)
    - [Frames & sentinels](./tutorial/fundamentals/03-frames-and-sentinels.md)
    - [Default codecs](./tutorial/fundamentals/04-default-codec.md)
    - [Custom decoder functions](./tutorial/fundamentals/05-custom-decoder.md)
    - [Implementing `DecodeValue`](./tutorial/fundamentals/06-decode-value-impl.md)
    - [Value lengths](./tutorial/fundamentals/07-val-lengths.md)
    - [Latebind transforms](./tutorial/fundamentals/08-latebind.md)
    - [Encoding & the `&` sigil](./tutorial/fundamentals/09-encode-sigil.md)
    - [Optional fields & default](./tutorial/fundamentals/10-default-fallback.md)

# Tutorial - complex behaviour

- [Advanced](./tutorial/advanced/README.md)
    - [Nested packets](./tutorial/advanced/11-nested-packets.md)
    - [Repeated extraction](./tutorial/advanced/12-repeated-extraction.md)
    - [Break conditions](./tutorial/advanced/13-break-condition.md)
    - [Async / Tokio streams](./tutorial/advanced/14-tokio-streams.md)
    - [Streaming decode with `Decoder<T>`](./tutorial/advanced/15-streaming-decode.md)

# Reference

- [Container attributes](./reference/container-attributes.md)
- [Field attributes](./reference/field-attributes.md)
- [Sigil coercion & `EncodeAs`](./reference/sigil-coercion.md)
- [Traits](./reference/traits.md)
- [Codecs](./reference/codecs.md)

# Architecture

- [Overview](./architecture/overview.md)

# Appendix

- [Contributing](./contributing.md)
