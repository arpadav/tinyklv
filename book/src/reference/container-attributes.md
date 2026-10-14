# Container attributes

Every attribute below is declared inside the `#[klv(...)]` on the struct
definition.

## `stream = <type>`

```rust,ignore
#[klv(stream = &[u8])]
```

Names the input type `S` the codecs read, a `winnow::Stream + StreamIsPartial`.
Defaults to `&[u8]` and is usually written only for clarity, or for `&str`
text-based KLV dialects. A type with a lifetime or generics is quoted:
`stream = "&'a [u8]"`. A value decoder is handed `S::Slice` for a value whose
bytes the derive takes first, and `S` for one it reads from the main stream (an
`Inline` item, or `size(implicit)` without `exact`), so on a stream whose
`Slice` is another type it must take both.

## `sentinel = <elements of stream>`

```rust,ignore
#[klv(sentinel = b"\xBE\xEF")]
```

Declares the magic bytes that mark the start of a packet. Adding a sentinel
causes the derive to implement `SeekSentinel` and, via a blanket impl,
`DecodeFrame`. It is a byte string searched for in the input's bytes, so on a
`&str` stream it is a compile error.

## `break_on = <key literal | path>`

```rust,ignore
#[klv(break_on = 0xFF)]
#[klv(break_on = classify)] // fn classify(key: &u8, input: &mut &[u8]) -> BreakType
```

Checked right after each key (under `keylen`, after the header). A key
literal ends the decode there, leaving the terminator's length and value
([Limits](./limits.md#a-literal-leaves-the-terminators-length)). A path gets the
key and the input after it, to read on: `Proceed` decodes the item from just
after the key, `Skip` moves on
and `Done` stops where the path left the input, and `Abort` is an error. Under
`keylen`, a `Skip` or `Done` that read nothing steps over a `Sliced` value, or
stops after an `Inline` header. On a short read, return `Proceed`, so streaming
waits. To check a decoded value, use the field-level
[`break_on`](./field-attributes.md#break_on--path).

## `key(dec = ..., enc = ...)`

```rust,ignore
#[klv(key(dec = decb::u8, enc = encb::u8))]
```

Associates the codec pair used for the **key** byte(s) of every KLV triple in
this container.

## `len(dec = ..., enc = ..., size(...))`

```rust,ignore
#[klv(len(dec = decb::u8_as_usize, enc = encb::u8_from_usize, size(exact = 1)))]
```

The length is a `usize`, which `dec` outputs and `enc` accepts: use the
`_as_usize` / `_from_usize` helpers from `decb` / `encb`. `len(size(exact = N))`
declares an `N`-byte slot, back-filled after a value of unknown width, so `enc`
must write `N` bytes; a release build does not check. A value too long for the
slot is left out of the encode
([Limits](./limits.md#values-too-long-for-their-length)). A `len(..)` with no
`exact` is variable-width, for BER-style codecs.

## `keylen(dec = ..., enc = ...)`

```rust,ignore
#[klv(keylen(dec = dec_header, enc = enc_header))]
```

One codec for the whole item header, in place of `key(..)` + `len(..)`, so
the key decides each value's framing at run time. `dec` matches
`fn(&mut S) -> tinyklv::Result<(K, Framing)>`: `Framing::Sliced(n)` hands the
field decoder the next `n` bytes as a sub-slice, `Framing::Inline` hands it the
main stream to read until it stops itself. `enc` matches
`fn((K, usize), &mut Vec<u8>)`, takes no sigil, and is given each key with the
length of its encoded value. A failed header read is truncation: streaming
waits for more bytes, one-shot stops.

Field decoders return `tinyklv::Result<T>`, and a `size(lengthed)` one is
passed a `Sliced` item's length; an `Inline` item for it is an error.
`keylen(..)` takes no `key(..)`, `len(..)` or `sentinel`, and no field takes
`size(implicit)`: each is a compile error. Without
[`unknown(..)`](#unknowndec---enc--), a key no field declares is skipped when
`Sliced` and ends the decode when `Inline`.

## `default(typ = T, dec = ..., enc = ..., size(...), skip_if...)`

```rust,ignore
#[klv(
    default(typ = u8,  dec = decb::u8,     enc = *encb::u8),
    default(typ = u16, dec = decb::be_u16, enc = *encb::be_u16),
    // an encoder only
    default(typ = MyStruct, enc = MyStruct::encode_value),
    // a size and a skip for every `u64` field
    default(
        typ = u64, dec = dec_varint, enc = *enc_varint,
        size(implicit), skip_if_default,
    ),
)]
```

Attaches a codec to a concrete field type: every field of type `T` (or
`Option<T>`) takes from it whichever of `dec`, `enc`, `size(..)` and skip it
leaves unset, each meaning what it does on a field. `typ` and one of `dec` /
`enc` are required. `lengthed` follows the `dec` the field uses, and an
inherited `implicit` cannot be turned off. A second `default(typ = T, ..)`
adds to the first; one setting given twice for `T` is a compile error.

## `debug`

```rust,ignore
#[klv(debug)]
```

Enables generated runtime decode logging for matched fields and decoded key/value
pairs. With the `tracing` feature this emits `tracing::debug!`; otherwise it falls
back to `println!`.

## `deny_unknown_keys`

```rust,ignore
#[klv(deny_unknown_keys)]
```

A key no field declares is an error.

## `unknown(dec = ..., enc = ...)`

```rust,ignore
#[klv(unknown(dec = dec_frame, enc = &enc_frame))]
```

The codec of every item whose key no field declares. `dec` takes the key
first, as a `lengthed` decoder takes the length:
`fn(K) -> impl Fn(&mut S) -> tinyklv::Result<U>`. `enc` matches
`fn(&U, &mut Vec<u8>)` and takes the field sigils. In key + len, `dec` reads
everything after the key, its length included, and `enc` writes it back. Beside
`keylen(..)`, `dec` reads the value as the header framed it, and `enc` writes
the value alone.

Decoded values are dropped unless a
[`#[klv(unknown)]`](./field-attributes.md#unknown) field keeps them, and `enc`
is required only beside that field. Without it, a `keylen` struct skips a
`Sliced` unknown item undecoded. It is a compile error beside
`deny_unknown_keys`. The decoder's contract:
[19 - Unknown items](../tutorial/advanced/19-unknown.md).

## `strict`

```rust,ignore
#[klv(strict)]
```

A value sized by its header is an error when its decoder fails or leaves part
unread, labelled `` `S::f` value (its decoder rejected the item's value) `` or
`left bytes of the item's value unread`, the cursor past the item.
`08 05 1A 02 FF FE`, an `id` then a `name: Option<String>` its decoder
rejects, is an error; without `strict` it decodes, `name` keeping what it had:
`None`. What it misses:
[Limits](./limits.md#what-strict-does-not-check).

## `allow_unimplemented_encode`

```rust,ignore
#[klv(allow_unimplemented_encode)]
```

Lets an encoder be missing, and then `EncodeValue` / `EncodeFrame` are not
emitted; with every encoder given, they still are.

## `allow_unimplemented_decode`

```rust,ignore
#[klv(allow_unimplemented_decode)]
```

Symmetric counterpart for decoders and the decode impls.

## `trait_fallback`

```rust,ignore
#[klv(trait_fallback)]
```

Fields lacking explicit `enc`/`dec` and not matched by a
container `default(..)` fall back to the `EncodeValue`/`DecodeValue`
trait impls on the field type.

## Cross-reference

| Attribute | First introduced in |
|-----------|--------------------|
| `stream` | [03 - Frames & sentinels](../tutorial/fundamentals/03-frames-and-sentinels.md) |
| `sentinel` | [03 - Frames & sentinels](../tutorial/fundamentals/03-frames-and-sentinels.md) |
| `key(dec, enc)` | [01 - First packet](../tutorial/fundamentals/01-first-packet.md) |
| `len(dec, enc, size)` | [01 - First packet](../tutorial/fundamentals/01-first-packet.md) |
| `keylen(dec, enc)` | [18 - Framing per key](../tutorial/advanced/18-keylen.md) |
| `break_on` | [14 - Break conditions](../tutorial/advanced/14-break-condition.md) |
| `default(typ, dec, enc, size)` | [04 - Default codecs](../tutorial/fundamentals/04-default-codec.md) |
| `unknown(dec, enc)` | [19 - Unknown items](../tutorial/advanced/19-unknown.md) |
| `allow_unimplemented_encode` | [01 - First packet](../tutorial/fundamentals/01-first-packet.md) |
| `trait_fallback` | [11 - Optional fields & default](../tutorial/fundamentals/11-default-fallback.md) |
