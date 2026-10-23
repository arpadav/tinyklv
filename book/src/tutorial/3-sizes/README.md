# Sizes

| page | concept | needs |
|---|---|---|
| [3.1 Value sub-slices](./01-the-value-is-a-sub-slice.md) | a decoder gets only its value's bytes | 1.1 |
| [3.2 Lengthed decoders](./02-lengthed-decoders.md) | `size(lengthed)`: the decoder is passed the length | 1.1, 2.3 |
| [3.3 Exact widths](./03-exact-widths.md) | `size(exact = N)`: the value is always N bytes | 1.1, 3.1 |
| [3.4 Reserve hints](./04-reserve-hints.md) | `size(reserve = N)`: the encoder sets aside N bytes | 1.1, 3.2 |
| [3.5 Values with no length](./05-values-with-no-length.md) | `size(implicit)`: no length is written | 1.1, 3.3 |
| [3.6 Fixed-width lengths](./06-length-slot-width.md) | `len(.., size(exact = W))`: every length is W bytes | 1.1, 3.2 |
| [3.7 Framing per key](./07-framing-per-key.md) | `keylen`: the key says whether a length follows | 1.1, 3.5 |
