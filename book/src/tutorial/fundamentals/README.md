# Fundamentals

Core features of `tinyklv`, from first decode to full round-trip.

| # | Tutorial | Concept |
|---|----------|---------|
| 01 | [First packet](./01-first-packet.md) | KLV triple structure, basic `DecodeValue` |
| 02 | [Out-of-order & prelude](./02-out-of-order.md) | Keyed dispatch, `prelude::*` |
| 03 | [Frames & sentinels](./03-frames-and-sentinels.md) | `DecodeFrame`, sentinel seeking |
| 04 | [Default codecs](./04-default-codec.md) | Container-level `default(typ = T, ...)` |
| 05 | [Custom decoder functions](./05-custom-decoder.md) | Writing a `fn(&mut S) -> Result<T>` |
| 06 | [Implementing DecodeValue](./06-decode-value-impl.md) | Trait-based codec on the type |
| 07 | [Value lengths](./07-val-lengths.md) | Subslices and `varlen = true` |
| 08 | [Latebind transforms](./08-latebind.md) | Post-decode `Fn(T) -> U` and `Fn(&mut T)` |
| 09 | [Encoding & sigils](./09-encode-sigil.md) | `EncodeValue`, `&` and `*` sigils |
| 10 | [Macros](./10-macros.md) | Macros for `enc`/`dec` |
| 11 | [Optional fields & default](./11-default-fallback.md) | `Option<T>`, `default`, `trait_fallback` |
