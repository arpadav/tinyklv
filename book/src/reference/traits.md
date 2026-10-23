# Traits

`use tinyklv::prelude::*` brings every trait below into scope. `#[derive(Klv)]` implements them:

| trait | does | implemented when the struct has |
|---|---|---|
| [`DecodeValue`](#decodevalue) | reads a value | a decoder for every field |
| [`EncodeValue`](#encodevalue) | writes a value | an encoder for every field |
| [`DecodePartial`](#decodepartial) | reads what has arrived, and says if more is needed | a decoder for every field |
| [`Partial`](#partial) | turns the partly read struct into the struct | a decoder for every field |
| [`DecodeFrame`](#decodeframe) | finds a packet by its sentinel, and reads it | a `sentinel` and every decoder |
| [`DrainFrames`](#drainframes) | reads packets until one fails | a `sentinel` and every decoder |
| [`EncodeFrame`](#encodeframe) | writes a whole packet | a `sentinel` and every encoder |
| [`EncodeAs`](./sigils.md#encodeas) | the view the `&` sigil hands an encoder | never: tinyklv implements it on std types |

## `DecodeValue`

```rust,ignore
pub trait DecodeValue<S>: Sized
where
    S: winnow::stream::Stream,
{
    fn decode_value(input: &mut S) -> tinyklv::Result<Self>;
}
```

Reads one value off the front of the input, and moves the input past it. On a derived struct, it reads items until the input ends or a `break_on` stops it. Implement it on a type of your own to use as a decoder ([2.2 Codec traits](../tutorial/2-codecs/02-value-traits.md)).

> [!NOTE]
> **A `Vec` of a type that implements it.** `Vec<T>` implements it too: it reads values until one fails or reads nothing, and returns those before it. A derived struct reads items until the input ends, so two packets back to back, with no sentinel, decode as one element that holds the second packet's values. A `Vec` of packets needs packets that end themselves, as at a `break_on`.

## `EncodeValue`

```rust,ignore
pub trait EncodeValue {
    fn encode_value(&self, out: &mut Vec<u8>);
}
```

Appends the value's bytes to `out`. On a derived struct, that is its items, with no sentinel and no packet length. `Vec<T>` implements it when T does, writing each element in turn.

## `DecodeFrame`

```rust,ignore
pub trait DecodeFrame<S>: Sized
where
    S: winnow::stream::Stream,
{
    fn decode_frame(input: &mut S) -> tinyklv::Result<Self>;
}
```

Skips to the sentinel, reads the packet's length with `len`'s decoder, and reads that many bytes with `decode_value`. Once it finds a whole packet, it moves the input past it, even when the body fails to decode.

## `DrainFrames`

```rust,ignore
pub trait DrainFrames<S>: Sized
where
    S: winnow::stream::Stream,
{
    fn drain_frames(input: &mut S) -> tinyklv::Result<Vec<Self>>;
}
```

Calls `decode_frame` until one fails, and returns the packets before it. It does not return that failure: the input is left just after the last packet it returns ([Limits](./limits.md#drain_frames-stops-at-a-bad-packet)).

## `EncodeFrame`

```rust,ignore
pub trait EncodeFrame {
    fn encode_frame(&self, out: &mut Vec<u8>);
}
```

Appends a whole packet: the sentinel, the body's length by `len`'s encoder, then what `encode_value` writes.

## `DecodePartial`

```rust,ignore
pub trait DecodePartial<S>: Sized + DecodeValue<S>
where
    S: winnow::stream::Stream,
{
    type Partial: Default + Partial<Final = Self>;
    fn decode_partial(
        input: &mut S,
    ) -> Result<Packet<Self, Self::Partial>, PartialError>;
}

pub enum Packet<T, P>
where
    P: Partial<Final = T>,
{
    Ready(T),
    NeedMore(P),
}
```

Reads what has arrived of one body, with no sentinel:

| it returns | when |
|---|---|
| `Ready`, with the struct | decoding ended before the input did, such as at a `break_on` |
| `NeedMore`, with the fields read so far | the input ran out |
| an error | an item was malformed or rejected, or a required field is missing at the end |

On `NeedMore`, wait for more bytes, then continue from its fields, p:

```rust,ignore
let packet = Sensor::resume_partial(&mut input, p)?;
```

If the body is already whole, `finalize` on p returns the struct instead. A `PartialError` has two methods:

- `kind`: its `ErrorKind`
- `label`: what was rejected, which is also what it prints

## `Partial`

```rust,ignore
pub trait Partial: Sized {
    type Final;
    fn finalize(self) -> Result<Self::Final, PartialError>;
}
```

The derive writes `<Name>PartialPacket`, a copy of the struct with every field an `Option`. `finalize` checks that every required field is in, and returns the struct; `try_into` does the same.

## `Decoder`

A buffer for a stream of packets with a sentinel: feed it bytes as they arrive, and take each packet once all its bytes are in.

```rust,ignore
let mut decoder = Sensor::decoder();
decoder.feed(&bytes);                       // from a socket, a file, ..
while let Some(sensor) = decoder.next() {   // every packet whose bytes are in
    // ..
}
```

| method | does |
|---|---|
| `feed` | appends bytes |
| `next` | returns the next whole packet, or `None` |
| `iter` | `next`, as an iterator |
| `buffered` | the bytes not yet decoded |
| `clear` | drops every buffered byte |
| `finish` | ends the stream, and says whether it ended cleanly: see the note |
| `next_fresh` | as `next`, but returns why no packet came out |

`next_fresh` is a method of the trait `PartialIterator`:

```rust,ignore
let sensor = decoder.next_fresh()?;    // Err: a DecodeIterError
```

Its error is a `DecodeIterError`:

| `DecodeIterError` | means |
|---|---|
| `Eof` | the buffer is empty |
| `NeedMore` | a packet has not all arrived |
| `Malformed` | a packet failed to decode: it holds the error, and the packet's bytes are dropped |

> [!NOTE]
> **Why `next` returned `None`.** No whole packet is buffered yet, or the next one is malformed: its bytes are dropped, and `next` returns `None` though packets may follow. See [Limits](./limits.md#decodernext-stops-at-a-bad-packet).
>
> **At the end of a stream.** Call `next` until no packet comes out, then `finish`. It returns `Ok(None)` when `buffered` is empty. Bytes left there are a packet cut short, or one `next` did not decode. `finish` drops them, and returns an error, `ErrorKind::Truncated`.

## `BreakType`

```rust,ignore
#[non_exhaustive]
pub enum BreakType {
    Proceed,
    Skip,
    Done,
    Abort(&'static str),
}
```

What a `break_on` function returns. Its meaning differs on [a container](./container-attributes.md#a-function) and on [a field](./field-attributes.md#break_on).

## `Framing`

```rust,ignore
#[non_exhaustive]
pub enum Framing {
    Sliced(usize),
    Inline,
}
```

What a `keylen` decoder returns with each key: see [`keylen`](./container-attributes.md#keylen).

## `ErrorKind`

A decode error is winnow's `ContextError`. To act on why it failed, read its kind:

```rust,ignore
let kind = ErrorKind::of(&error);    // Option<ErrorKind>: None when untagged
```

| `ErrorKind` | the decode failed because |
|---|---|
| `Truncated` | the input ends inside a key, a length or a value |
| `MalformedValue` | a value's bytes are not valid for its type |
| `WrongLength` | an item's length is one its value cannot have |
| `NoProgress` | an item read no bytes, so it would be read again forever |
| `UnknownKey` | a key no field declares, under `deny_unknown_keys` |
| `MissingRequired` | a required field has no value at the end |
| `Aborted` | a `break_on` returned `Abort` |
| `MissingSentinel` | no sentinel begins a packet in the input |

A decoder of yours can tag its own errors, so callers can read the kind back:

```rust,ignore
return Err(ErrorKind::MalformedValue.tag(error));
```

> [!NOTE]
> **Matching on it.** It is `non_exhaustive`: a match on it needs a catch-all `_` arm.
>
> **Comparing errors.** Compare their kinds, not the errors: the kind is stored in the error as a label, so two errors of one kind from different places are not equal.
