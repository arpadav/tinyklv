# Container attributes

Every attribute below is declared inside the `#[klv(...)]` on the struct
definition. Each section gives the syntax, a one-paragraph semantic summary,
and a minimal snippet showing the attribute in context.

## `stream = <type>`

```rust,ignore
#[klv(stream = &[u8])]
```

Names the input type that every codec in this container consumes. Defaults to
`&[u8]` and is usually written only for clarity or when the stream type is
something else (for example, `&str` for text-based KLV dialects).

## `sentinel = <elements of stream>`

```rust,ignore
#[klv(sentinel = b"\xBE\xEF")]
```

Declares the magic bytes that mark the start of a frame. Adding a sentinel
causes the derive to implement `SeekSentinel` and, via a blanket impl,
`DecodeFrame`. Without a sentinel, only `DecodeValue` is produced.

Usually, since stream defaults to `&[u8]`, a byte-string is used here. However,
if the stream is `&str` then a `&'static str` can be used.

## `key(dec = ..., enc = ...)`

```rust,ignore
#[klv(key(dec = decb::u8, enc = encb::u8))]
```

Wires the codec pair used for the **key** byte(s) of every KLV triple in
this container. Supply `dec` for decoding support and `enc` for encoding
support; either half can be omitted to stay unimplemented.

Encoder sigils on keys are **omitted** - not yet implemented, no current need.

## `len(dec = ..., enc = ...)`

```rust,ignore
#[klv(len(dec = decb::u8_as_usize, enc = encb::u8_from_usize))]
```

The length value will *always* be `usize`. Therefore, `dec` must output a `usize`
and `enc` must accept a `usize`. Use the `_as_usize` / `_from_usize` variants from `decb` / `encb`.

Encoder sigils len are **omitted** - not yet implemented, no current need.

## `default(typ = T, dec = ..., enc = ..., varlen = <bool>)`

```rust,ignore
#[klv(
    default(typ = u8,  dec = decb::u8,     enc = encb::u8),
    default(typ = u16, dec = decb::be_u16, enc = encb::be_u16),
    default(typ = MyStruct, dec = MyStruct::decode_value, MyStruct::encode_value),
)]
```

Attaches a codec to a concrete field type. Every field of type `T` in the
struct resolves its codec from this default unless it overrides locally.
`varlen` is optional and selects the variable-length decoder shape; see
[`varlen`](./field-attributes.md).

## `debug`

```rust,ignore
#[klv(debug)]
```

Prints the derive's expansion to stderr at compile time. Useful when
debugging codec wiring or diagnosing an attribute parse error.

## `deny_unknown_keys`

```rust,ignore
#[klv(deny_unknown_keys)]
```

Makes the generated `decode_value` return an error when it encounters a key
it does not have a field for. Without it, unknown keys are skipped silently
for forward compatibility.

## `allow_unimplemented_encode`

```rust,ignore
#[klv(allow_unimplemented_encode)]
```

Opts out of the encoder half of the derive. `EncodeValue` / `EncodeFrame`
are not emitted. Useful for decode-only consumers of a third-party spec.

## `allow_unimplemented_decode`

```rust,ignore
#[klv(allow_unimplemented_decode)]
```

Symmetric counterpart: opts out of `DecodeValue` / `DecodeFrame`. Use when
the struct is an emitter-only producer.

## Cross-reference

| Attribute | First introduced in |
|-----------|--------------------|
| `stream` | [Frames & sentinels](../tutorial/03-frames-and-sentinels.md) |
| `sentinel` | [Frames & sentinels](../tutorial/03-frames-and-sentinels.md) |
| `key(dec, enc)` | [First packet](../tutorial/01-first-packet.md) |
| `len(dec, enc)` | [First packet](../tutorial/01-first-packet.md) |
| `default(typ, dec, enc, varlen)` | [Default codecs](../tutorial/04-default-codec.md) |
| `debug` | Reference only |
| `deny_unknown_keys` | Reference only |
| `allow_unimplemented_encode` | [First packet](../tutorial/01-first-packet.md) |
| `allow_unimplemented_decode` | Reference only |
