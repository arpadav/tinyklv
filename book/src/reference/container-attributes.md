# Container attributes

Every attribute below is declared inside the `#[klv(...)]` on the struct
definition. Each section gives the syntax, a one-paragraph semantic summary,
and a minimal snippet showing the attribute in context.

## `stream = <type>`

```rust,ignore
#[klv(stream = &[u8])]
```

Names the input type that every codec in this container consumes. Any
`winnow::Stream` type works. Defaults to `&[u8]` and is usually written
only for clarity, or when the stream is something else (for example,
`&str` for text-based KLV dialects, or a custom buffered-reader type).

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

Associates the codec pair used for the **key** byte(s) of every KLV triple in
this container. Supply `dec` for decoding support and `enc` for encoding
support; either half can be omitted to stay unimplemented.

## `len(dec = ..., enc = ..., size(...))`

```rust,ignore
#[klv(len(dec = decb::u8_as_usize, enc = encb::u8_from_usize, size(exact = 1)))]
```

The decoded length value is always `usize`; `dec` must output a `usize` and `enc`
must accept a `usize`. Use the `_as_usize` / `_from_usize` helpers from `decb` /
`encb`. `len(size(exact = N))` declares a fixed-width `N`-byte length prefix.
A bare `len(..)` or `len(size(var))` uses the variable-width length path, which
is the path used by BER-style length codecs.

## `default(typ = T, dec = ..., enc = ..., size(...))`

```rust,ignore
#[klv(
    default(typ = u8,  dec = decb::u8,     enc = *encb::u8),
    default(typ = u16, dec = decb::be_u16, enc = *encb::be_u16),
    // `dec` and `enc` are independently optional: attach an encoder
    // only, and decoders for `MyStruct` can still be used per-field
    default(typ = MyStruct, enc = MyStruct::encode_value),
)]
```

Attaches a codec to a concrete field type. Every field of type `T` in the
struct resolves its codec from this default unless it overrides locally.
Only `typ` is required; `dec`, `enc`, and `size(..)` are independently
optional, so a default can supply just a decoder, just an encoder, or both.
`size(var)` selects the length-taking decoder shape; `size(exact = N)` and
`size(hint = N)` carry the same encode meanings as field-level `size(..)`. A
default `size(..)` applies only to fields that resolve through that default.

The `enc` path inside `default(...)` accepts the same `&` and `*` sigils
as field-level `enc`. See
[Sigil coercion & `EncodeAs`](./sigil-coercion.md).

## `debug`

```rust,ignore
#[klv(debug)]
```

Enables generated runtime decode logging for matched fields and decoded key/value
pairs. With the `tracing` feature this emits `tracing::debug!`; otherwise it falls
back to `println!` in the generated decode code. Encode overflow diagnostics are
separate error-path logging and are not controlled by this flag.

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

## `trait_fallback`

```rust,ignore
#[klv(trait_fallback)]
```

Opt-in flag. Fields lacking explicit `enc`/`dec` and not matched by a
container `default(..)` fall back to the `EncodeValue`/`DecodeValue`
trait impls on the field type. Field-level codecs and container
`default(..)` both take precedence over the fallback.

## Cross-reference

| Attribute | First introduced in |
|-----------|--------------------|
| `stream` | [03 - Frames & sentinels](../tutorial/fundamentals/03-frames-and-sentinels.md) |
| `sentinel` | [03 - Frames & sentinels](../tutorial/fundamentals/03-frames-and-sentinels.md) |
| `key(dec, enc)` | [01 - First packet](../tutorial/fundamentals/01-first-packet.md) |
| `len(dec, enc, size)` | [01 - First packet](../tutorial/fundamentals/01-first-packet.md) |
| `default(typ, dec, enc, size)` | [04 - Default codecs](../tutorial/fundamentals/04-default-codec.md) |
| `debug` | Reference only |
| `deny_unknown_keys` | Reference only |
| `allow_unimplemented_encode` | [01 - First packet](../tutorial/fundamentals/01-first-packet.md) |
| `allow_unimplemented_decode` | Reference only |
| `trait_fallback` | [11 - Optional fields & default](../tutorial/fundamentals/11-default-fallback.md) |
