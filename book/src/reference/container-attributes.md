# Container attributes

| attribute | sets | tutorial |
|---|---|---|
| [`key`](#key) | the codec of every key | [1.1 First packet](../tutorial/1-basics/01-first-packet.md) |
| [`len`](#len) | the codec of every length | [3.6 Fixed-width lengths](../tutorial/3-sizes/06-length-slot-width.md) |
| [`keylen`](#keylen) | one codec for each key and its length | [3.7 Framing per key](../tutorial/3-sizes/07-framing-per-key.md) |
| [`sentinel`](#sentinel) | the bytes that start every packet | [1.3 Frames and sentinels](../tutorial/1-basics/03-frames-and-sentinels.md) |
| [`stream`](#stream) | the input type every decoder reads | [1.3 Frames and sentinels](../tutorial/1-basics/03-frames-and-sentinels.md) |
| [`default`](#default) | the codecs of every field of one type | [1.4 Default codecs](../tutorial/1-basics/04-default-codecs.md) |
| [`trait_fallback`](#trait_fallback) | a field's codecs from its type's traits | [2.6 Trait fallback](../tutorial/2-codecs/06-trait-fallback.md) |
| [`unknown`](#unknown) | the codec of every item no field declares | [5.2 Keeping unknown items](../tutorial/5-unknown/02-keep-them.md) |
| [`deny_unknown_keys`](#deny_unknown_keys) | a key no field declares is an error | [5.1 Skipping or denying unknown keys](../tutorial/5-unknown/01-skip-or-deny.md) |
| [`strict`](#strict) | a value read only in part is an error | [5.3 Strict decoding](../tutorial/5-unknown/03-strict-values.md) |
| [`break_on`](#break_on) | a check on each key, which can stop decoding | [6.1 Break on a key](../tutorial/6-control/01-stop-on-a-key.md) |
| [`debug`](#debug) | a log of each item decoded | [8.1 Debug logging](../tutorial/8-tooling/01-debug-logging.md) |
| [`allow_unimplemented_encode`](#allow_unimplemented_encode-allow_unimplemented_decode) | a struct that only decodes | [8.2 Decode-only and encode-only](../tutorial/8-tooling/02-one-direction-only.md) |
| [`allow_unimplemented_decode`](#allow_unimplemented_encode-allow_unimplemented_decode) | a struct that only encodes | [8.2 Decode-only and encode-only](../tutorial/8-tooling/02-one-direction-only.md) |

## `key`

`key` is a container attribute: the codec of every key (K). A field's `key` is a K.

### Format

```rust,ignore
key(
    dec = ..,    // path to the decoder: fn(&mut S) -> Result<K>
    enc = ..,    // path to the encoder: fn(K, &mut Vec<u8>)
    size(..),    // optional, below
)
```

| `size(..)` | effect |
|---|---|
| `exact = N` or `reserve = N` | the encoder sets aside N bytes per key; no byte changes |

The encoder takes no sigil: it already receives the key by value.

### Example

```rust,ignore
#[klv(
    key(
        dec = decb::be_u16,
        enc = encb::be_u16,
        size(exact = 2),             // 2 bytes set aside per key
    ),
    // len(..) as on page 1.1
)]
struct WideKey {
    #[klv(
        key = 0x0102,
        dec = decb::u8,
        enc = *encb::u8,
    )]
    seq: u8,
}
```

Byte representation of key 0x0102: <span class="klv">k 01 02</span>

> [!NOTE]
> **A key encoder of another width.** On a key, `size(exact = N)` checks nothing: a key is as wide as its encoder writes. With `size(exact = 2)` and the 1-byte `u8` encoder, every key is still 1 byte. Pick the key codec for the width you want.

## `len`

`len` is a container attribute: the codec of every length (L). A length is always a `usize`.

### Format

```rust,ignore
len(
    dec = ..,    // path to the decoder: fn(&mut S) -> Result<usize>
    enc = ..,    // path to the encoder: fn(usize, &mut Vec<u8>)
    size(..),    // optional, below
)
```

| `size(..)` | effect |
|---|---|
| `exact = W` | every length is W bytes |
| `reserve = N` | the encoder sets aside N bytes per length; no byte changes |

The encoder takes no sigil: it already receives the length by value.

### Example

```rust,ignore
#[klv(
    len(
        dec = decb::u8_as_usize,
        enc = encb::u8_from_usize,
        size(exact = 1),             // every length is 1 byte
    ),
)]
```

Byte representation of a length of 2: <span class="klv">l 02</span>

> [!NOTE]
> **A value too long for its length.** With `size(exact = 1)` a length is one byte, so it can say at most 255. A longer value, say a 300-byte string, does not fit the length's byte budget set by `exact`, and a debug build panics:
>
> ```text
> tinyklv: value of 300 bytes does not fit the 1-byte length budget set by `size(exact = 1)`
> ```
>
> A release build leaves that whole item out (key, length and value) and writes the rest of the packet. It logs the same message, ending in `; omitting the item`, through `tracing::error!` (stderr without the feature).
>
> **The encoder's width.** Under `size(exact = W)` the length encoder must write exactly W bytes. For a value whose size is only known at run time, such as a string, a debug build checks this, and the encode panics if not. A release build does not check, and writes the extra bytes over the value: under `size(exact = 1)`, `ber_length` writes a length of 200 as 2 bytes, the second in place of the value's first byte. The item then reads as cut short, and the field as missing.
>
> **No `size(..)`.** Nothing then checks a length against its encoder. Given 300, `u8_from_usize` panics in a debug build. A release build writes 44, which is 300 mod 256, then all 300 bytes of the value, and the packet still decodes, wrongly. Take page 3.2's struct with a name of 300 "x": the decoder reads the first 44 bytes as the name, then the other 256 as more items, of key 78 ("x"), which no field declares. It skips two, finds the third cut short, and stops there with no error: the name comes back as 44 bytes, and 12 bytes stay in the input.

## `keylen`

`keylen` is a container attribute: one codec for each item's key and length together, in place of the two. The key can then decide whether a length follows.

### Format

```rust,ignore
keylen(
    dec = ..,    // path to the decoder: fn(&mut S) -> Result<(K, Framing)>
    enc = ..,    // path to the encoder: fn((K, usize), &mut Vec<u8>)
)
```

The decoder returns each key with the framing of its value:

| `Framing` | the field's decoder gets |
|---|---|
| `Sliced(n)` | the next n bytes, as a sub-slice |
| `Inline` | the bytes that follow; it must stop itself |

The encoder is given each key with its value's length. It writes the key, and the length where the key needs one. It takes no sigil.

### Example

The key's top bit says whether a length byte follows:

```rust,ignore
{{#include ../../../examples/book/ref/book_ref_keylen.rs:header}}

#[klv(
    keylen(
        dec = dec_header,
        enc = enc_header,
    ),
)]
struct Sensor {
    #[klv(
        key = 0x01,                  // top bit clear: no length
        dec = decb::u8,
        enc = *encb::u8,
    )]
    seq: u8,

    #[klv(
        key = 0x82,                  // top bit set: a length follows
        dec = decb::be_u16,
        enc = *encb::be_u16,
    )]
    temp: Option<u16>,
}
```

```klv
k 01        // 1. top bit clear: Inline
v 2A        // 2. u8 reads 1 byte, and stops
k 82        // 3. top bit set: dec_header reads the length
l 02        // 4. Sliced(2): the next 2 bytes go to be_u16
v 09 2E     // 5. be_u16 reads 2350
```

### Overview

| beside `keylen` | result |
|---|---|
| `key(..)` or `len(..)` | compile error: `keylen` replaces both |
| a `size(..)` inside `keylen(..)` | compile error: the header decides each framing |
| `sentinel` | compile error: no codec for the packet's length |
| a field's `size(implicit)` | compile error: the header says whether a length follows |
| a field's `size(exact = N)` | a `Sliced` length other than N is an error, `ErrorKind::WrongLength` |
| a field's `size(lengthed)` | its decoder is passed a `Sliced` value's length; an `Inline` value is an error |

> [!NOTE]
> **One framing per key.** The decoder of a value is picked by its key alone. Return the same framing for a key every time: a decoder that reads to the end of its input reads past its own value when its key comes `Inline`.
>
> **A key no field declares.** It is skipped when `Sliced`. When `Inline`, there is no length to skip it by, so decoding stops before it, and the rest stays in the input. With [`unknown`](#unknown), its decoder reads such an item instead.
>
> **A header that fails to read.** It is taken as cut short: `decode_value` returns the fields read before it, and a streaming decode waits for more bytes. So a broken header in a whole packet is never reported as an error; check that the input is empty after decoding.
>
> **A `keylen` struct with a sentinel.** `keylen` has no length codec for a whole packet, so the sentinel brings its own; see [`sentinel`](#sentinel). Without it, the build fails.

## `sentinel`

`sentinel` is a container attribute: the bytes that mark the start of every packet. With it, the struct gains:

| method | does |
|---|---|
| `encode_frame` | writes the sentinel, the body's length, then the body |
| `decode_frame` | skips to the sentinel, reads the length, decodes that many bytes |
| `drain_frames` | decodes packets until one fails |
| `decoder()` | a streaming [`Decoder`](./traits.md#decoder) |

The body's length is written and read with [`len`](#len)'s codec, or with the codec given in the sentinel's own `len`.

### Format

```rust,ignore
sentinel = b"..",        // a byte string

sentinel(
    value = b"..",       // a byte string
    len(..),             // the body's length codec, written as `len`: optional
                         // with `key` and `len`, required with `keylen`
)
```

### Example

```rust,ignore
#[klv(
    sentinel = b"SENSOR",
    // key(..), len(..) as on page 1.1
)]
```

Byte representation of page 1.1's struct, after junk:

```klv
j DE AD 00 00             // junk: decode_frame skips it
s 53 45 4E 53 4F 52       // "SENSOR"
l 07                      // the body's length
    k 01 l 01 v 2A        // seq  = 42
    k 02 l 02 v 09 2E     // temp = 2350
```

> [!NOTE]
> **A body too long for its length.** The body's length follows the same rules as a value's. With `size(exact = 1)` on it, a body over 255 bytes does not fit, even when each item in it fits. A debug build panics:
>
> ```text
> tinyklv: packet body of 307 bytes does not fit the 1-byte length budget set by `size(exact = 1)`
> ```
>
> A release build writes nothing at all, not even the sentinel, and logs the same message, ending in `; omitting the packet`. With no `size(..)`, `u8_from_usize` panics in a debug build, and a release build writes the length mod 256.

## `stream`

`stream` is a container attribute: the input type S that every decoder of the struct reads. It defaults to `&[u8]`.

### Format

```rust,ignore
stream = S,    // default: &[u8]; &str for a text protocol
```

### Example

A type with a lifetime goes in quotes. With it, a field can borrow its bytes from the input, with no copy:

```rust,ignore
#[klv(
    stream = "&'a [u8]",
    // key(..), len(..) as on page 1.1
)]
struct Raw<'a> {
    #[klv(
        key = 0x03,
        dec = borrow_value,          // fn(&mut &'a [u8]) -> Result<&'a [u8]>
        enc = *put_bytes,            // fn(&[u8], &mut Vec<u8>)
    )]
    payload: &'a [u8],
}
```

Decoding <span class="klv">k 03 l 04 v 72 6F 6F 66</span> sets `payload` to the input's own 4 value bytes.

> [!NOTE]
> **A `&str` stream.** Every codec of the struct then reads a `&str`, the key's and the length's included, so the built-in codecs, which read `&[u8]`, no longer fit. A sentinel is searched for in bytes, so beside a `&str` stream it is a compile error.

## `default`

`default` is a container attribute: the codecs of every field of one type, so each such field needs only its key.

### Format

```rust,ignore
default(
    typ = T,            // the field type it applies to
    dec = ..,           // as a field's dec
    enc = ..,           // as a field's enc
    size(..),           // optional: as a field's size
    skip_if_default,    // optional: or skip_if = ..
)
```

`typ` is required, and so is a decoder or an encoder. A field of type T, or `Option<T>`, takes each setting it leaves unset:

| setting | a field of type T takes it when |
|---|---|
| `dec` | it has no `dec` |
| `enc` | it has no `enc` |
| `size(implicit)` | always: a field cannot turn it off |
| `size(exact = N)` | it has no `exact` and no `reserve` |
| `size(reserve = N)` | it has no `exact` and no `reserve` |
| `size(lengthed)` | it uses this `dec` |
| `skip_if_default` or `skip_if` | it has neither |

### Example

```rust,ignore
#[klv(
    default(
        typ = String,
        dec = decs::to_string_utf8,
        enc = &encs::from_string_utf8,
        size(lengthed),
    ),
    // key(..), len(..) as on page 1.1
)]
struct Labeled {
    #[klv(
        key = 0x03,                  // dec, enc, size: the String default
    )]
    label: String,
}
```

Byte representation of "roof": <span class="klv">k 03 l 04 v 72 6F 6F 66</span>

> [!NOTE]
> **A type written another way.** A default applies to a field whose type is written exactly as its `typ`, or to an `Option` of it. A type alias is another name, so a field of that type takes nothing from the default.
>
> **Two defaults for one type.** The second adds its settings to the first. A setting both give is a compile error.
>
> **An `Option` as the type.** A compile error: an `Option` field already takes the default of the type it holds.

## `trait_fallback`

`trait_fallback` is a container attribute: a field with no codec of its own, and none from a `default`, uses its type's [codec traits](./traits.md).

### Format

```rust,ignore
trait_fallback,
```

Each of `dec` and `enc` comes from the first of these that has one:

| order | the codec comes from |
|---|---|
| 1 | the field's own `dec` or `enc` |
| 2 | the container's `default` for the field's type |
| 3 | `trait_fallback`: the type's codec traits |

### Example

```rust,ignore
#[klv(
    trait_fallback,
    // key(..), len(..) as on page 1.1
)]
struct Fallback {
    #[klv(
        key = 0x02,                  // dec, enc: Celsius's traits
    )]
    temp: Celsius,                   // page 2.2's Celsius
}
```

Byte representation of 23.5 °C: <span class="klv">k 02 l 02 v 09 2E</span>

> [!NOTE]
> **An integer field.** The integers, such as `u16`, implement neither trait, so the fallback cannot fill them. Give them their own codecs, or a `default`.
>
> **A `lengthed` field.** `decode_value` takes no length, so a `lengthed` field needs its own decoder. Without one, it is a compile error.

## `unknown`

`unknown` is a container attribute: the codec of every item whose key no field declares. A field marked [`unknown`](./field-attributes.md#unknown) keeps what it decodes; without one, the decoded items are dropped.

### Format

```rust,ignore
unknown(
    dec = ..,    // path to the decoder: fn(K) -> impl Fn(&mut S) -> Result<U>
    enc = ..,    // path to the encoder: fn(&U, &mut Vec<u8>)
)
```

The decoder is passed the key, as a `lengthed` decoder is passed the length. The encoder takes a sigil, as a field's does, and is needed only beside the field that keeps the items. What each one handles:

| framing | the decoder reads | the encoder writes |
|---|---|---|
| `key` and `len` | the length, then the value | the length, then the value |
| `keylen` | the value, as the header framed it | the value: `keylen` writes the header |

### Example

```rust,ignore
#[klv(
    unknown(
        dec = dec_unknown,           // reads the length, then the value
        enc = &enc_unknown,          // writes them back
    ),
    // key(..), len(..) as on page 1.1
)]
```

```klv
k 01 l 01 v 2A        // seq  = 42
u 07 01 FF            // key 07, no field: dec_unknown(0x07) reads 01 FF
k 02 l 02 v 09 2E     // temp = 2350
```

> [!NOTE]
> **Without `unknown`.** With `key` and `len`, an unknown item is skipped by its length. With `keylen`, one that is `Sliced` is skipped. One that is `Inline` stops the decode.
>
> **An item cut short.** When the input ends inside an item, the decoder must move the input to its end before it returns the error, as the full example does with `finish`. A streaming decode then waits for more bytes, instead of rejecting the packet. A failure before the end of the input is an error, `ErrorKind::MalformedValue`.
>
> **Beside `deny_unknown_keys`.** A compile error: one rejects the items the other reads.

## `deny_unknown_keys`

`deny_unknown_keys` is a container attribute: a key no field declares is an error, `ErrorKind::UnknownKey`. Without it, such an item is skipped.

### Format

```rust,ignore
deny_unknown_keys,
```

### Example

Page 1.1's struct with `deny_unknown_keys`, decoding:

```klv
k 01 l 01 v 2A        // seq  = 42
u 07 01 FF            // no field has key 07: an error
k 02 l 02 v 09 2E     // not read
```

> [!NOTE]
> **Beside `unknown`.** A compile error: one rejects the items the other reads.

## `strict`

`strict` is a container attribute: a value its decoder rejects, or reads only in part, is an error, `ErrorKind::MalformedValue`.

### Format

```rust,ignore
strict,
```

| a value's decoder | without `strict` | with `strict` |
|---|---|---|
| leaves bytes unread | they are dropped | an error |
| fails | the field is left as it was | an error |

### Example

Page 1.1's struct with `strict`, decoding page 3.1's padded temp:

```klv
k 01 l 01 v 2A             // seq  = 42
k 02 l 04 v 09 2E 00 00    // be_u16 reads 2 of the 4 bytes: an error
```

> [!NOTE]
> **A field left as it was.** Without `strict`, a field whose value fails keeps what it had: an `Option` stays empty. A required field with no other item is then missing, `ErrorKind::MissingRequired`.
>
> **What it does not check.** `strict` checks each value that has a length, not the input as a whole. See [Limits](./limits.md#what-strict-does-not-check).

## `break_on`

`break_on` is a container attribute: a check on each key, right after it is read, which can stop the decode there. To check a decoded value instead, see the field's [`break_on`](./field-attributes.md#break_on).

### Format

```rust,ignore
break_on = K,    // a key literal
break_on = f,    // or a path to fn(&K, &mut S) -> BreakType
```

- [A key literal](#a-key-literal): stop at that key
- [A function](#a-function): decide per key

### A key literal

Decoding stops right after the key, and leaves the rest of its item in the input:

```klv
k 01 l 01 v 2A        // seq  = 42
k 02 l 02 v 09 2E     // temp = 2350
k FF                  // 1. key FF: decoding stops
l 00                  // 2. left in the input,
k 01 l 01 v 2B        //    with the next packet
```

The encoder writes no end item: no field has its key.

### A function

The function gets the key and the input right after it. What it returns tells the decoder what to do:

| `BreakType` | the decoder |
|---|---|
| `Proceed` | decodes the item, from just after the key |
| `Skip` | goes on from where the function left the input, without decoding the item |
| `Done` | stops where the function left the input |
| `Abort(msg)` | stops with an error, `ErrorKind::Aborted`, labelled msg |

This one reads the end item's length and value before it stops, so the next packet starts the input:

```rust,ignore
{{#include ../../../examples/book/ref/book_ref_break_on.rs:end_item}}
```

```klv
k FF l 00             // 1. end_item reads the length and value, then Done
k 01 l 01 v 2B        // 2. left in the input: the next packet
```

> [!NOTE]
> **`Done` with a field missing.** `Done` returns the fields decoded so far. A required field not among them is an error, `ErrorKind::MissingRequired`.
>
> **A short read.** If the function reads past the key and the input ends, return `Proceed`: the item then reads as cut short, and a streaming decode waits for more bytes.
>
> **Under `keylen`.** If the function read nothing, `Skip` and `Done` first step over the value, when it has a length. An `Inline` value has no length to step over, so the decode ends after its key. Read such a value in the function before you return `Skip`.

## `debug`

`debug` is a container attribute: it logs each item as it decodes, to find the item a packet fails on.

### Format

```rust,ignore
debug,
```

| `tracing` feature | the log goes to |
|---|---|
| off | stdout, by `println!` |
| on | `tracing::debug!` |

### Example

Page 1.1's struct with `debug` prints, as it decodes page 1.1's bytes:

```text
key: 1, len: 1
	seq: Ok(42)
key: 2, len: 2
	temp: Ok(2350)
```

## `allow_unimplemented_encode`, `allow_unimplemented_decode`

Container attributes for a struct that only decodes, or only encodes.

### Format

```rust,ignore
allow_unimplemented_encode,    // encoders may be missing
allow_unimplemented_decode,    // decoders may be missing
```

| attribute | may be missing | then not implemented |
|---|---|---|
| `allow_unimplemented_encode` | any encoder, of a field, a key or a length | `EncodeValue`, `EncodeFrame` |
| `allow_unimplemented_decode` | any decoder, of a field, a key or a length | `DecodeValue` and every decode method |

With every encoder given, the encode traits are still implemented; the same holds for decoders.

### Example

```rust,ignore
#[klv(
    key(
        dec = decb::u8,              // no enc
    ),
    len(
        dec = decb::u8_as_usize,
    ),
    allow_unimplemented_encode,
)]
struct Listener {
    #[klv(
        key = 0x01,
        dec = decb::u8,
    )]
    seq: u8,
}
```

> [!NOTE]
> **A missing encoder without the attribute.** A compile error, whose message names `allow_unimplemented_encode` as one fix. The same holds for a decoder.


## Run this example

```sh
cargo run --example book_ref_container
cargo run --example book_ref_size
cargo run --example book_ref_keylen
cargo run --example book_ref_unknown
cargo run --example book_ref_break_on
```

<details>
<summary>Full example: key, sentinel, stream, default, trait_fallback, debug, allow_unimplemented_encode</summary>

```rust
{{#include ../../../examples/book/ref/book_ref_container.rs}}
```

</details>

<details>
<summary>Full example: len</summary>

```rust
{{#include ../../../examples/book/ref/book_ref_size.rs}}
```

</details>

<details>
<summary>Full example: keylen</summary>

```rust
{{#include ../../../examples/book/ref/book_ref_keylen.rs:all}}
```

</details>

<details>
<summary>Full example: unknown, deny_unknown_keys, strict</summary>

```rust
{{#include ../../../examples/book/ref/book_ref_unknown.rs}}
```

</details>

<details>
<summary>Full example: break_on</summary>

```rust
{{#include ../../../examples/book/ref/book_ref_break_on.rs:all}}
```

</details>
