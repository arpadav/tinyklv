# Field attributes

| attribute | sets | tutorial |
|---|---|---|
| [`key`](#key) | the key of the field's item | [1.1 First packet](../tutorial/1-basics/01-first-packet.md) |
| [`oneof`](#oneof) | several keys, one per variant of an enum | [4.4 One-of fields](../tutorial/4-presence/04-one-of-several-keys.md) |
| [`dec`](#dec) | the field's decoder | [2.1 Custom decoders](../tutorial/2-codecs/01-your-own-decoder.md) |
| [`enc`](#enc) | the field's encoder | [2.3 Sigils](../tutorial/2-codecs/03-sigils.md) |
| [`size`](#size) | how the field's length is written, read and used | [3.2 Lengthed decoders](../tutorial/3-sizes/02-lengthed-decoders.md) to [3.5 Values with no length](../tutorial/3-sizes/05-values-with-no-length.md) |
| [`default`](#default) | the value when the key is missing | [4.2 Default values](../tutorial/4-presence/02-fallback-values.md) |
| [`skip_if_default`](#skip_if_default-skip_if) | no item when the field is at its default | [4.3 Skipping defaults](../tutorial/4-presence/03-skipping-defaults.md) |
| [`skip_if`](#skip_if_default-skip_if) | no item when a function says so | [4.3 Skipping defaults](../tutorial/4-presence/03-skipping-defaults.md) |
| [`latebind`](#latebind) | a function run on the decoded value | [2.5 Latebind](../tutorial/2-codecs/05-latebind.md) |
| [`break_on`](#break_on) | a check on the decoded value, which can stop decoding | [6.3 Break on a field value](../tutorial/6-control/03-decide-per-value.md) |
| [`unknown`](#unknown) | keeps every item no field declares | [5.2 Keeping unknown items](../tutorial/5-unknown/02-keep-them.md) |

## `key`

`key` is a field attribute: the key of the field's item, a value of the container's key type K.

### Format

```rust,ignore
key = K,            // a literal
key = oneof(..),    // or one key per variant, below
```

### Example

```rust,ignore
#[klv(
    key = 0x02,
    dec = decb::be_u16,
    enc = *encb::be_u16,
)]
temp: u16,
```

Byte representation of the key: <span class="klv">k 02</span>

> [!NOTE]
> **Two fields with one key.** A compile error: the items of that key would decode into only one of them.
>
> **A field with no `#[klv(..)]`.** It is neither read nor written. Decoding sets it to its type's `Default`, so that type must implement it.

### `oneof`

`oneof` routes several keys to one field, each to a variant of the field's enum.

```rust,ignore
key = oneof(
    K1 => Enum::A(T1),    // an item of key K1 holds a T1, kept as Enum::A
    K2 => Enum::B(T2),
),
```

- the field is the enum, or an `Option` of it
- each arm's value uses its type's codec, from the container's [`default`](./container-attributes.md#default) or [`trait_fallback`](./container-attributes.md#trait_fallback)
- decoding: the last of these items wins
- encoding: writes the set variant under its key, in the field's place

#### Example

```rust,ignore
#[klv(
    default(
        typ = u16,
        dec = decb::be_u16,
        enc = *encb::be_u16,
    ),
    // key(..), len(..) as on page 1.1
)]
struct OneOf {
    #[klv(
        key = oneof(
            0x02 => Temp::Celsius(u16),
            0x03 => Temp::Fahrenheit(u16),
        ),
    )]
    temp: Temp,
}
```

Byte representation of 74.30 °F, under key 03: <span class="klv">k 03 l 02 v 1D 06</span>

| beside `oneof` | result |
|---|---|
| a `dec` or an `enc` | compile error: each arm uses its type's codec |
| a `size(..)` | compile error, for the same reason |
| two arms of one variant | compile error when the struct encodes: a variant is written under one key |

## `dec`

`dec` is a field attribute: the field's decoder.

### Format

```rust,ignore
dec = f,    // fn(&mut S) -> Result<T>
dec = f,    // with size(lengthed): fn(usize) -> impl Fn(&mut S) -> Result<T>
```

In both, f is a path, a macro such as [`scale!`](./codecs.md#macros), or an expression in parentheses, such as a closure. For a value with a length, its input is just the value's bytes; for an `implicit` value, the rest of the input.

### Example

Temp in whole °C, with closures:

```rust,ignore
#[klv(
    key = 0x02,
    dec = (|input: &mut &[u8]| {
        decb::be_u16(input).map(|centi| centi / 100)
    }),
    enc = (|temp: &u16, out: &mut Vec<u8>| {
        encb::be_u16(temp * 100, out);
    }),
)]
temp: u16,
```

Byte representation of 23 °C, written as 2300: <span class="klv">k 02 l 02 v 08 FC</span>

> [!NOTE]
> **A field with no `dec`.** It takes the decoder of the container's [`default`](./container-attributes.md#default) for its type, or its type's trait under [`trait_fallback`](./container-attributes.md#trait_fallback). With neither, the build fails, unless the container sets `allow_unimplemented_decode`.

## `enc`

`enc` is a field attribute: the field's encoder, with an optional [sigil](./sigils.md).

### Format

```rust,ignore
enc = f,     // fn(&T, &mut Vec<u8>): a reference to the field
enc = *f,    // fn(T, &mut Vec<u8>): a copy of the field
enc = &f,    // fn(T::Borrowed<'_>, &mut Vec<u8>): its EncodeAs view
```

In each, f is a path, a macro such as [`scale_enc!`](./codecs.md#macros), or an expression in parentheses.

### Example

```rust,ignore
#[klv(
    key = 0x02,
    dec = decb::be_u16,
    enc = *encb::be_u16,             // be_u16 takes a u16: a copy
)]
temp: u16,
```

Byte representation of 2350: <span class="klv">k 02 l 02 v 09 2E</span>

> [!NOTE]
> **A field with no `enc`.** It takes the encoder of the container's [`default`](./container-attributes.md#default) for its type, or its type's trait under [`trait_fallback`](./container-attributes.md#trait_fallback). With neither, the build fails, unless the container sets `allow_unimplemented_encode`.

## `size`

`size` is a field attribute. It describes the byte representation of what it is applied to.

### Format

```rust,ignore
size(input, ..)
```

Each input is optional; the [Overview](#overview) lists how they combine:

- [`implicit`](#implicit): no length is written
- [`exact = N`](#exact--n): the value is always N bytes
- [`lengthed`](#lengthed): the decoder is passed the length
- [`reserve = N`](#reserve--n): the encoder sets aside N bytes

### `implicit`

`implicit` means no length is written, since the key _implies_ the value's length.

This affects encoding and decoding:

- the byte representation changes from KLV to KV
- the encoder does not write L
- the decoder takes the value straight after the key: the key selects the field, and the field's `dec` reads just its value

Normal KLV, <span class="klv">k 01 l 02 v 01 02</span>:

```klv
k 01        // 1. read the key: selects field n
l 02        // 2. read the length: 2 bytes follow
v 01 02     // 3. take 2 bytes: be_u16 reads 0x0102
```

With `implicit`, <span class="klv">k 01 v 01 02</span>:

```klv
k 01        // 1. read the key: selects field n
v 01 02     // 2. the key's decoder maps to be_u16, which always takes 2 bytes
```

#### Example

```rust,ignore
#[klv(
    key = 0x01,
    dec = decb::be_u16,
    enc = *encb::be_u16,
    size(implicit),
)]
n: u16,                      // 0x0102
```

Byte representation: <span class="klv">k 01 v 01 02</span>

### `exact = N`

`exact = N` means the value is always N bytes. Its length is then redundant ([`implicit`](#implicit) drops it), but a standard may still write it, so a decoder can skip unknown items. `exact = N` keeps it, for speed: the encoder writes N without measuring the value, and the decoder checks it.

> [!NOTE]
> When decoding, a length other than N is an error, `ErrorKind::WrongLength`.
>
> When encoding, the field's encoder must write exactly N bytes. If it writes more or fewer, a debug build panics inside `encode_value`. A release build does not check: it writes the length the encoder really produced, and the packet then fails to decode.

### `lengthed`

`lengthed` means the field's decoder is passed the length. Use it for a value only its length can size, such as a `String`. It changes the decoder's signature and nothing else: the length comes first.

```rust,ignore
// tinyklv::dec::string
pub fn to_string_utf8(len: usize) -> impl Fn(&mut &[u8]) -> tinyklv::Result<String>

#[klv(
    key = 0x01,
    dec = decs::to_string_utf8,  // takes len: usize
    enc = &encs::from_string_utf8,
    size(lengthed),
)]
s: String,
```

> [!NOTE]
> `implicit` and `lengthed` together are a compile error. An implicit value has no length, so there is nothing to pass to the decoder. Add `exact = N` and the derive passes it N.

### `reserve = N`

`reserve = N` only speeds up encoding. The encoder sets aside N bytes up front for a value of dynamic length, such as a `String`; no byte changes.

### Overview

<div class="wide">

| `size(..)` | encode | decode | byte representation |
|---|---|---|---|
| none | key, length, value | reads the length, takes that many bytes | <span class="klv-k">K</span> <span class="klv-l">L</span> <span class="klv-v">V</span> |
| `exact = N` | key, N, value | the length must be N; takes N bytes | <span class="klv-k">K</span> <span class="klv-l">L</span> <span class="klv-v">V</span> |
| `lengthed` | key, length, value | takes the length's bytes; passes the length to `dec` | <span class="klv-k">K</span> <span class="klv-l">L</span> <span class="klv-v">V</span> |
| `exact = N, lengthed` | key, N, value | the length must be N; takes N bytes; passes N to `dec` | <span class="klv-k">K</span> <span class="klv-l">L</span> <span class="klv-v">V</span> |
| `implicit` | key, value | `dec` reads until it stops | <span class="klv-k">K</span> <span class="klv-v">V</span> |
| `implicit, exact = N` | key, value | takes N bytes | <span class="klv-k">K</span> <span class="klv-v">V</span> |
| `implicit, exact = N, lengthed` | key, value | takes N bytes; passes N to `dec` | <span class="klv-k">K</span> <span class="klv-v">V</span> |
| `reserve = M`, beside any but `exact` | sets aside M bytes first | unchanged | unchanged |
| `implicit, lengthed` | compile error: no length to pass | | |
| `exact = N, reserve = M` | compile error: the width is already known | | |

</div>

> [!NOTE]
> A container's [`default(..)`](./container-attributes.md#default) can give every field of a type a `size`. A field takes some of its inputs even when it sets its own `size`; that entry lists which.

## `default`

`default` is a field attribute: the value the field takes when its key is missing from the packet.

### Format

```rust,ignore
default,           // the type's Default::default()
default = expr,    // or this expression
```

### Example

```rust,ignore
#[klv(
    key = 0x02,
    dec = decb::be_u16,
    enc = *encb::be_u16,
    default = 2000,                  // a missing temp is 2000
)]
temp: u16,
```

Decoding <span class="klv">k 01 l 01 v 2A</span>, with no temp, gives a temp of 2000.

> [!NOTE]
> **A missing field with no `default`.** It is an error, `ErrorKind::MissingRequired`. An `Option` field is the exception: it is then `None`.
>
> **Encoding.** The field is always written, at its default too. To leave it out, see [`skip_if_default`](#skip_if_default-skip_if).

## `skip_if_default`, `skip_if`

Field attributes that leave the field's item out of the encode. A missing item then decodes to its type's `Default`, unless the field has its own [`default`](#default).

### Format

```rust,ignore
skip_if_default,    // when the field equals its type's Default::default()
skip_if = f,        // when f, a fn(&T) -> bool, returns true
```

A field takes one of the two, once. A container's [`default`](./container-attributes.md#default) can set one for every field of a type; a field's own replaces it.

### Example

```rust,ignore
#[klv(
    key = 0x02,
    dec = decb::be_u16,
    enc = *encb::be_u16,
    skip_if_default,                 // no item when temp is 0
)]
temp: u16,

#[klv(
    key = 0x03,
    dec = decs::to_string_utf8,
    enc = &encs::from_string_utf8,
    size(lengthed),
    skip_if = String::is_empty,      // no item when label is empty
)]
label: String,
```

With a seq of 42, a temp of 0 and an empty label, the struct encodes to <span class="klv">k 01 l 01 v 2A</span> alone.

> [!NOTE]
> **An `Option` field.** The test takes the whole field: `None` is never written, and a `Some` of 0 is.
>
> **A float of -0.0.** It equals 0.0, so `skip_if_default` leaves it out, and the field decodes as 0.0.

## `latebind`

`latebind` is a field attribute: a function run on what `dec` returns, when decoding only.

### Format

```rust,ignore
latebind = f,         // fn(T) -> F: what dec returns, into the field's type F
latebind = &mut f,    // fn(&mut T): changes it in place; the field is a T
```

### Example

```rust,ignore
#[klv(
    key = 0x02,
    dec = decb::be_u16,
    enc = *encb::be_u16,
    latebind = &mut clamp,           // caps temp at 12500
)]
temp: u16,
```

Decoding <span class="klv">k 02 l 02 v 32 C8</span>, 13000, gives a temp of 12500.

> [!NOTE]
> **Encoding.** `latebind` does not run: the encoder writes the field as it is, so a temp of 13000 is written as 13000.

## `break_on`

`break_on` is a field attribute: a function run on the field's value once it decodes, which can stop the decode. To check each key instead, see the container's [`break_on`](./container-attributes.md#break_on).

### Format

```rust,ignore
break_on = f,    // fn(&T) -> BreakType
```

| `BreakType` | the decoder |
|---|---|
| `Proceed` | keeps the value, and goes on |
| `Skip` | drops this item's value, and goes on |
| `Done` | keeps the value, and stops |
| `Abort(msg)` | stops with an error, `ErrorKind::Aborted`, labelled msg |

### Example

```rust,ignore
#[klv(
    key = 0x02,
    dec = decb::be_u16,
    enc = *encb::be_u16,
    break_on = check_temp,           // Abort above 12500
)]
temp: u16,
```

Decoding <span class="klv">k 02 l 02 v 32 C8</span>, 13000, is an error, `ErrorKind::Aborted`.

> [!NOTE]
> **Encoding order.** Fields with a `break_on` are written last, in the order they are declared, so a `Done` on one does not cut off the fields after it. The full example declares temp first, and it is still written after seq.
>
> **Two fields that return `Done`.** The first to return it stops the decode, so the other is never read. Let at most one field return `Done`.
>
> **An `Abort` in a streaming decode.** Its error carries only your message, not the field's name, so name the field in the message.

## `unknown`

`unknown` is a field attribute: the field keeps every item whose key no field declares, as a key and a value, in the order they came. The container's [`unknown`](./container-attributes.md#unknown) decodes each value.

### Format

```rust,ignore
#[klv(unknown)]
extra: Vec<(K, U)>,    // K: the key type; U: what unknown's dec returns
```

- one such field per struct, beside the container's `unknown`, with no other setting
- encoding writes the kept items after the other fields, before those with a `break_on`

### Example

```rust,ignore
#[klv(unknown)]
extra: Vec<(u8, Vec<u8>)>,
```

```klv
k 01 l 01 v 2A        // seq  = 42
u 07 01 FF            // kept: key 07, value FF
k 02 l 02 v 09 2E     // temp = 2350
```

The encoder writes the kept item after seq and temp:

```klv
k 01 l 01 v 2A        // seq  = 42
k 02 l 02 v 09 2E     // temp = 2350
u 07 01 FF            // the kept item
```

> [!NOTE]
> **Another collection.** The field's type can be your own, if it has:
>
> - `Default`
> - a `push` that takes a key and value pair
> - an `IntoIterator` for a reference to it
>
> One that drops items past a limit bounds what a packet can make you keep.
>
> **An item its decoder fails on.** It is an error, `ErrorKind::MalformedValue`, naming this field.

## Run this example

```sh
cargo run --example book_ref_field
cargo run --example book_ref_size
cargo run --example book_ref_break_on
cargo run --example book_ref_unknown
```

<details>
<summary>Full example: oneof, dec, enc, default, skip_if_default, skip_if, latebind</summary>

```rust
{{#include ../../../examples/book/ref/book_ref_field.rs}}
```

</details>

<details>
<summary>Full example: size</summary>

```rust
{{#include ../../../examples/book/ref/book_ref_size.rs}}
```

</details>

<details>
<summary>Full example: break_on</summary>

```rust
{{#include ../../../examples/book/ref/book_ref_break_on.rs:all}}
```

</details>

<details>
<summary>Full example: unknown</summary>

```rust
{{#include ../../../examples/book/ref/book_ref_unknown.rs}}
```

</details>
