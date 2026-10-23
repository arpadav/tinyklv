# Codecs

| page | concept | needs |
|---|---|---|
| [2.1 Custom decoders](./01-your-own-decoder.md) | your own function as a decoder | 1.1 |
| [2.2 Codec traits](./02-value-traits.md) | the codec on the type: `DecodeValue`, `EncodeValue` | 2.1 |
| [2.3 Sigils](./03-sigils.md) | how a field reaches its encoder: none, `*` or `&` | 1.1, 2.2 |
| [2.4 Macros](./04-macros.md) | built-in codec macros: `scale!`, `cast!` and more | 2.1, 2.3 |
| [2.5 Latebind](./05-latebind.md) | convert or change a value after decoding | 2.1 |
| [2.6 Trait fallback](./06-trait-fallback.md) | a field with no codec uses its type's traits | 1.4, 2.2 |
| [2.7 Nested packets](./07-nested-packets.md) | a derived struct as a field's value | 2.2 |
