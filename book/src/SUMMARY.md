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
    - [Macros](./tutorial/fundamentals/10-macros.md)
    - [Optional fields & default](./tutorial/fundamentals/11-default-fallback.md)

# Tutorial - complex behaviour

- [Advanced](./tutorial/advanced/README.md)
    - [Nested packets](./tutorial/advanced/12-nested-packets.md)
    - [Repeated extraction](./tutorial/advanced/13-repeated-extraction.md)
    - [Break conditions](./tutorial/advanced/14-break-condition.md)
    - [Streaming partial packets](./tutorial/advanced/15-streaming-decode.md)
    - [Async / Tokio streams](./tutorial/advanced/16-tokio-streams.md)

# Reference

- [Container attributes](./reference/container-attributes.md)
- [Field attributes](./reference/field-attributes.md)
- [Sigil coercion & `EncodeAs`](./reference/sigil-coercion.md)
- [Traits](./reference/traits.md)
- [Codecs](./reference/codecs.md)

# Appendix

- [Contributing](./contributing.md)
