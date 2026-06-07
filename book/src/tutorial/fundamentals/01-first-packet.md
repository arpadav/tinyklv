# Tutorial 01 - First packet

Before we begin, some information about `tinyklv`'s proc-macro format.

## Derive macro

`tinyklv`'s main entry-point is its derive proc-macro `tinyklv::Klv`. When applying
`#[derive(Klv)]` onto a named-struct (e.g. one with fields), a series of traits
are attempted to be implemented, including both decoding and encoding.

## Container Attributes

In order to decode, the bare-minimum `Klv` requires is a key and a len decoder. This
will indicate how every key and every length value is decoded. Note, that keys can be 
more descriptive literals, e.g. `&'static str`, which is why the key decoder exists
to convert this into bytes (using ASCII? UTF8?). Usually, just bytes will do, and
the `#[klv(key(dec = ..)]` container attribute will use a binary decoder of some unsigned
int, varying precision, varying endianness. 

The `#[klv(len(..)]` attribute should always decode as `usize` and encode from `usize`. This
is because the parsers using `winnow` usually "take" bytes will always end up being
coerced into a `usize` via the [`ToUsize` trait](https://docs.rs/winnow/latest/winnow/stream/trait.ToUsize.html#tymethod.to_usize)
anyways. It allows the developer to know up-front that that conversion is happening.

[All container attributes here](../../reference/container-attributes.md)

## Field attributes

Same `#[klv(..)]` format is used, where here `#[klv(key = literal)]` is used to assign a key
to the specified field. Similarly, `#[klv(dec = ..)]` is used to point to a function or macro
which can decode the byte stream into its specified type.

No field-level length attribute is necessary for fixed-width decoders: they have
the value width baked into the function itself. Decoders that need the runtime
KLV length use `size(var)`, which is introduced in
[Value lengths](./07-val-lengths.md). Decoder function signatures are discussed
more in the [Custom decoder functions](./05-custom-decoder.md) tutorial.

[All field attributes here](../../reference/field-attributes.md)

## Trait implementation 

Decoding is driven by `DecodeValue::decode_value`, which takes `&mut &[u8]` and advances
the slice pointer in place.

[All traits here](../../reference/traits.md)

## Example

`Heartbeat` carries two fields: a one-byte `sequence` and a two-byte big-endian
`temperature_centideg`.

The container sets `key(dec = decb::u8)` and `len(dec = decb::u8_as_usize)` to
say "one byte for the key, one byte for the length". Each field picks its own value decoder
(`decb::u8`, `decb::be_u16`). 

In addition, you might see `allow_unimplemented_encode` here. This is a flag, where by
default, `tinyklv::Klv` requires both encoding and decoding to be implemented. 
[More on this here](../../reference/container-attributes.md#allow_unimplemented_encode)

Run this example: `cargo run --example book_01_first_packet`

```rust
{{#include ../../../../examples/book_01_first_packet.rs}}
```

## Note

For the first couple of examples, the stream is hand-built and annotated so you can
see every byte.

## Overview

- KLV triples are `key | length | value`; the derive decodes them together.
- `decode_value` consumes the slice in place. `DecodeValue` trait provides
this, and is imported from the prelude.
- `allow_unimplemented_encode` is a flag, which is used in this tutorial until
encoding is covered.

**Next:** [02 - Out-of-order & prelude](./02-out-of-order.md)
