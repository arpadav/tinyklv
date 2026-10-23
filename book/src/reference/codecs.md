# Codecs

The codecs tinyklv ships, for any decoder or encoder an attribute names. The book imports them as:

```rust,ignore
use tinyklv::dec::binary as decb;    // integers, floats, lengths
use tinyklv::enc::binary as encb;
use tinyklv::dec::ber as decber;     // BER lengths and BER-OIDs
use tinyklv::enc::ber as encber;
use tinyklv::dec::string as decs;    // text
use tinyklv::enc::string as encs;
```

## Signatures

```rust,ignore
// a decoder
fn(input: &mut &[u8]) -> tinyklv::Result<T>

// a lengthed decoder, for size(lengthed): the derive calls dec(len)(input)
fn(len: usize) -> impl Fn(&mut &[u8]) -> tinyklv::Result<T>

// an encoder: it takes the value, so a field uses the * sigil
fn(input: T, out: &mut Vec<u8>)
```

`tinyklv::Result` is winnow's `Result`. The string encoders take a `&str` instead, for the `&` sigil.

## Integers and floats

Each name is both a decoder, in `decb`, and an encoder, in `encb`:

| type | big-endian | little-endian |
|---|---|---|
| `u16` | `be_u16` | `le_u16` |
| `u32` | `be_u32` | `le_u32` |
| `u64` | `be_u64` | `le_u64` |
| `u128` | `be_u128` | `le_u128` |
| `i16` | `be_i16` | `le_i16` |
| `i32` | `be_i32` | `le_i32` |
| `i64` | `be_i64` | `le_i64` |
| `i128` | `be_i128` | `le_i128` |
| `f32` | `be_f32` | `le_f32` |
| `f64` | `be_f64` | `le_f64` |

A single byte has no order: its codecs are `u8` and `i8`. Every other type also has a native codec of its own name, such as `u16`. It uses the machine's byte order, so its bytes differ between machines: name the order for anything you send.

## Lengths

A length is a `usize`, which these decoders return and these encoders take:

| decoder | encoder | a length of |
|---|---|---|
| `u8_as_usize` | `u8_from_usize` | 1 byte |
| `be_u16_as_usize` | `be_u16_from_usize` | 2 bytes, big-endian |
| `le_u16_as_usize` | `le_u16_from_usize` | 2 bytes, little-endian |
| `be_u32_as_usize` | `be_u32_from_usize` | 4 bytes, big-endian |
| `le_u32_as_usize` | `le_u32_from_usize` | 4 bytes, little-endian |
| `be_u64_as_usize` | `be_u64_from_usize` | 8 bytes, big-endian |
| `le_u64_as_usize` | `le_u64_from_usize` | 8 bytes, little-endian |
| `be_u128_as_usize` | `be_u128_from_usize` | 16 bytes, big-endian |
| `le_u128_as_usize` | `le_u128_from_usize` | 16 bytes, little-endian |
| `decber::ber_length` | `encber::ber_length` | 1 byte below 128, more above: [BER](#ber) |

Each width also has a native one, such as `u16_as_usize`.

## Lengthed integers

Each integer and float decoder of a fixed byte order has a `lengthed` twin, such as `be_u16_lengthed`. It reads the whole value, whatever its length:

| value bytes | `be_u16_lengthed` reads |
|---|---|
| <span class="klv">v 2A</span> | 42: a shorter value is zero-extended |
| <span class="klv">v 09 2E</span> | 2350 |
| <span class="klv">v 00 09 2E</span> | 2350: a longer value keeps its low-order 2 bytes |

The encoder twin is called with a width, and writes exactly that many bytes, so a value can take a width no Rust type has:

```rust,ignore
#[klv(
    key = 0x04,
    dec = decb::be_u32_lengthed,           // passed the length: 3
    enc = *(encb::be_u32_lengthed(3)),     // writes 3 bytes
    size(exact = 3, lengthed),
)]
count: u32,
```

Byte representation of 70000: <span class="klv">k 04 l 03 v 01 11 70</span>

## BER

[ITU-T X.690](https://www.itu.int/rec/T-REC-X.690) codecs, in `decber` and `encber`. They need the `ber` feature, on by default:

| codec | for | 47 | 128 | 201 |
|---|---|---|---|---|
| `ber_length` | a length | <span class="klv">l 2F</span> | <span class="klv">l 81 80</span> | <span class="klv">l 81 C9</span> |
| `ber_oid` | a key or a value | <span class="klv">v 2F</span> | <span class="klv">v 81 00</span> | <span class="klv">v 81 49</span> |

- `ber_length` decodes a `usize`; its encoder takes any unsigned integer
- `ber_oid` reads and writes any unsigned integer type; it ends itself, so it suits `size(implicit)`
- `ber_length` rejects <span class="klv">l 80</span>, BER's indefinite length, as `ErrorKind::MalformedValue`

## Text

Every text decoder is lengthed: the field needs `size(lengthed)`. Every text encoder takes a `&str`.

| decoder | encoder | the bytes are |
|---|---|---|
| `to_string_utf8` | `from_string_utf8` | UTF-8; an invalid byte reads as U+FFFD |
| `to_string_utf8_strict` | | UTF-8; an invalid byte is an error |
| `to_string_utf16_be` | `from_string_utf16_be` | UTF-16, big-endian, with no byte order mark |
| `to_string_utf16_le` | `from_string_utf16_le` | UTF-16, little-endian, with no byte order mark |
| `to_string_ascii` | `from_string_ascii` | ASCII; on decode, any other byte is an error |

A UTF-16 value of an odd number of bytes is an error, `ErrorKind::WrongLength`. The ASCII pair needs the `ascii` feature, and its encoder writes the text's bytes as they are, with no check.

### Numbers as text

With the `ascii` feature, the text modules also read and write numbers as ASCII text. Each decoder reads exactly the value's bytes, which must all be the number:

| decoder | encoder | the bytes are |
|---|---|---|
| `u8` to `u128` | the same names | base 10; a 0 before other digits is an error |
| `i8` to `i128` | the same names | base 10, with an optional sign |
| `f32`, `f64` | the same names | a float, such as 2.5 |
| `hex_u8` to `hex_u128` | the same names | base 16; written lower-case |
| `alpha` | | letters, read as a `String` |
| `digit` | | digits, read as a `String` |
| `alphanumeric` | | letters and digits, read as a `String` |

## Macros

Each macro expands to a decoder or an encoder, written in place in `dec` or `enc`:

```rust,ignore
scale!(dec, T, factor)                         // dec's value as T, times factor
cast!(dec, T)                                  // dec's value as T
scale_enc!(enc, T, W, factor)                  // T / factor, as W, by enc
scale_offset_enc!(enc, T, W, factor, offset)   // (T - offset) / factor, as W
cast_enc!(enc, T, W)                           // T as W, by enc
as_date!(dec, format, len)                     // len bytes of text, as a NaiveDate
as_time!(dec, format, len)                     // the same, as a NaiveTime
as_datetime!(dec, format, len)                 // the same, as a NaiveDateTime
```

- each cast is Rust's `as`: a float cast to an integer drops its fraction
- the encode macros take the field by reference, so they need no sigil
- the last three need the `chrono` feature; their dec is a text decoder, such as `decs::to_string_utf8`, and format is a chrono format string

[2.4 Macros](../tutorial/2-codecs/04-macros.md) uses them.

## Run this example

```sh
cargo run --example book_ref_codecs
```

<details>
<summary>Full example: lengthed integers and BER</summary>

```rust
{{#include ../../../examples/book/ref/book_ref_codecs.rs}}
```

</details>
