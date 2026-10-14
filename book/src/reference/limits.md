# Limits

tinyklv decodes what the wire says and does not repair it: a malformed packet is
the generator's problem, not the parser's. A packet that fails is handed back,
its bytes left for you to skip or keep. The `Decoder` keeps a packet a chunk
boundary cut until a later `feed` completes it, and reports one that can never
decode once, then moves on.

## Malformed input

None of these is handled by tinyklv, by design.

### A length that never arrives

A packet length in range, but past anything your protocol sends, makes the
`Decoder` wait for that many bytes: a length of 2^31 waits for 2 GiB. Cap the
bytes buffered while no packet comes out, then `clear()`, or reject the length
as `MalformedValue`, and the `Decoder` reports the packet once and moves on:

```rust,ignore
fn packet_len(input: &mut &[u8]) -> tinyklv::Result<usize> {
    let len = decb::be_u32_as_usize(input)?;
    if len > MAX_PACKET {
        let mut error = ContextError::new();
        error.push(StrContext::Label("packet length past MAX_PACKET"));
        return Err(ErrorKind::MalformedValue.tag(error));
    }
    Ok(len)
}
```

### A header that fails to read

A `keylen` header that fails, such as a lone `80` BER length, reads as
truncation: one-shot decode finalizes what it has, streaming waits. So does a
key + len length, unless the struct has `unknown(..)`, which rejects one failing
short of the end of the input: with BER lengths, `01 01 0A 02 80 03 01 0C`
decodes `{ a: 10, b: None, c: None }`. Check that the input is empty after the
decode.

### What `strict` does not check

`strict` checks each value its header sizes, not the input as a whole, so it
misses:

- bytes after the top-level struct: `01 01 07 02` decodes `{ a: 7 }`, `02` left;
- an item cut at the end of a sentinel body: `AA BB 05 01 01 07 02 02` decodes
  `{ a: 7, b: 0 }`;
- a `size(implicit, exact = N)` value whose decoder fails, left unset;
- a `Vec` value's cut last element: a `Vec` of `be_u16` on `01 02 03` is `[258]`;
- a kept `Sliced` unknown item whose decoder returns `Ok` early.

Check that the input is empty after the decode, and element counts in your own
type.

### Non-canonical BER

The BER codecs accept non-minimal forms that still fit: padded OID groups
(`80 80 05` is 5), a long-form length for a short value (`81 05` is 5), and an
`FF` first length byte, which X.690 reserves. Emit minimal BER; reject the rest
in your own codec.

### An item that consumes nothing

In key + len, a key decoder that only peeks, beside a `size(implicit)` decoder
that consumes nothing, loops forever. A key decoder must consume its key, an
`implicit` one its value.

## Untrusted input

None of these is handled by tinyklv, by design.

### Recursion depth

A struct that contains itself, through `Box`, `Vec` or `Option`, decodes each
level by recursion, so deep enough nesting overflows the stack and aborts: in
release, a one-field struct overflows a 2 MiB thread, the std and tokio default,
at about 14,500 levels (58 KB of input). Cap untrusted packet sizes, or decode
on a thread with a larger stack.

### Kept unknown items

A `#[klv(unknown)]` field keeps every unknown item. Its type needs only
`Default`, `push((key, value))` and `&T: IntoIterator`, so a type of your own
can drop items past your limit.

### Lengths read from the wire

tinyklv's codecs allocate only for bytes that are there: `take(len)` borrows,
and a `Vec<T>` pre-allocates at most 4 KiB. A decoder of yours may trust a
length: `Vec::with_capacity(len)` on a length of 2^62 aborts. Size buffers from
the slice you are handed.

### Lengths past `usize`

The `*_as_usize` length decoders cast with `as`, so a `u128` length above
`usize::MAX`, or a `u64` one above 4 GiB on a 32-bit target, wraps: 2^64 + 5
reads as 5. For hostile peers, use a length decoder that fails through
`usize::try_from`.

### Values too long for their length

On encode, a value longer than its length codec can express is dropped, and
logged through `tracing` or `eprintln!`, when its length is back-filled into a
`len(size(exact = N))` slot, and written under a wrapped length (a debug build
panics) when a `*_from_usize` encoder writes it first. A relay decoding text
with the lossy `to_string_utf8`, which writes invalid bytes back as three-byte
`U+FFFD`s, can so grow a value and forge the next field. Size length codecs for
your largest value; relay text with `to_string_utf8_strict`.

### Reserve hints

A `size(reserve = N)` or `exact` total above `i32::MAX` reserves nothing; only
the hint is lost. Keep hints under 2 GiB.

## Streaming

None of these is handled by tinyklv, by design.

### `drain_frames` never fails

It stops at the first packet that fails, cut or malformed, returns the ones
before it, and leaves the input at that packet's sentinel, where
`T::decode_frame` returns the error.

### `Decoder::next` stops at a malformed packet

`next` returns `None` at a malformed packet, so a `while let Some` loop, or
`iter()`, stops with good packets still buffered behind it. `next_fresh()`
(`PartialIterator`) returns the error and moves on.

### A streamed `Vec` element cut short

`Vec<T>::decode_partial` pushes an element a chunk boundary cut once its
required fields are in, and its rest starts the next element: `01 01 07 02`
gives `[{ a: 7, b: None }]`, leaving `02`. Give each element a sentinel and use
the `Decoder`, or decode the `Vec` one-shot.

## `break_on`

### A literal leaves the terminator's length

A `break_on` literal ends the loop right after the key, leaving the
terminator's length and value (under `keylen`, its value). A `Vec` of such
structs reads them as its next element, and `strict` rejects a nested one:
`01 01 07 FF 00 01 01 08 FF 00` gives `[{ temp: Some(7) }, { temp: None }]`.
Read them in a fn before `Done`:

```rust,ignore
fn end(key: &u8, input: &mut &[u8]) -> BreakType {
    if *key != 0xFF {
        return BreakType::Proceed;
    }
    match winnow::binary::length_take(decb::u8_as_usize).parse_next(input) {
        Ok(_) => BreakType::Done,
        Err(_) => BreakType::Proceed,
    }
}
```

Under `keylen`, the fn's `Done` steps over a `Sliced` value it did not read.

### `Skip` and `Done` under `keylen`

A `Skip` or `Done` that read nothing steps over a `Sliced` value, but an
`Inline` one has no length, so the loop ends after its header. Read an `Inline`
value in the fn before `Skip`.

### `Proceed` on a short read under `keylen`

Without `unknown(..)`, a fn that returns `Proceed` on a short read hands the
loop the item, and an unknown `Inline` one ends it: with key `10` `Inline` and a
fn reading two bytes, `10 AA` is `Ready`, nothing decoded. Add `unknown(..)`,
and the loop waits.

### Field `break_on`

Fields with a `break_on` encode last, in declaration order, so when the first
returns `Done` the second is never decoded: let at most one return `Done`. A
streaming `Abort` error carries only your message: name the field in it.

## Codecs and attributes

### Error kinds

`ErrorKind::of` returns the outermost classification: under `strict`, a nested
struct missing a field and an odd-length UTF-16 value both read as
`MalformedValue`. The UTF-8, ASCII and date codecs tag none. The kind is stored
as a label, so it shows in `Debug` and `context()`, and takes part in `==`:
compare kinds, not errors.

### `exact` widths

`key(.., size(exact = N))` only sizes the encode reserve; the key's width is
what its encoder writes, so use a fixed-width key codec. A codec that normalizes
its value cannot round-trip under `size(exact = N)` once that changes its width.

### Fields at their default

tinyklv writes every field it encodes, where proto3 leaves out one at its
default. Add `skip_if_default` or `skip_if` to such fields. Not handled by
tinyklv, by design: presence on the wire is explicit.

### Names and attributes

- A `stream` type with a lifetime or generics is quoted: `stream = "&'a [u8]"`.
- The derive adds a `<Name>PartialPacket` type beside each struct it decodes,
  and `decoder()` to one with a `sentinel`: rename your item that collides.
- `sentinel` beside `keylen(..)` is a compile error, there being no packet
  length codec: make the `keylen` struct the one field of a key + len struct
  with the `sentinel`
  ([12 - Nested packets](../tutorial/advanced/12-nested-packets.md)).
