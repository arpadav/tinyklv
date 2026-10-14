# Codecs

Built-in codec paths shipped with `tinyklv`. Every path listed is a
drop-in value for `dec = ...`, `enc = ...`, `key(dec = ..., enc = ...)`,
or `len(dec = ..., enc = ...)`.

Tutorials import these modules as:

```rust,ignore
use tinyklv::dec::binary as decb;
use tinyklv::enc::binary as encb;
use tinyklv::dec::string as decs;
use tinyklv::enc::string as encs;
```

## Function signatures

Every codec path satisfies one of the shapes below. A decoder takes one of
two, selected by `size(lengthed)` on the field:

```rust,ignore
// without size(lengthed)
fn(input: &mut S) -> tinyklv::Result<T>

// with size(lengthed): the derive calls dec_fn(len)(input)
fn(len: usize) -> impl Fn(&mut S) -> tinyklv::Result<T>
```

`tinyklv::Result` is an alias for `winnow::Result`. The `_lengthed`
binary variants and the string codecs satisfy the length-parameterized
shape.

An encoder appends bytes to a caller-owned buffer and returns nothing:

```rust,ignore
fn(val: &T, out: &mut Vec<u8>)
```

The `&` and `*` sigils rewrite how the field is handed in (`*` for a
value-taking `fn(T, &mut Vec<u8>)`); see
[Sigil coercion & `EncodeAs`](./sigil-coercion.md).

## Binary (`tinyklv::dec::binary` / `tinyklv::enc::binary`)

Fixed-width integer and float codecs in three endianness families:
native (`u8`, `u16`, ...), big-endian (`be_u16`, ...), and
little-endian (`le_u16`, ...). For single-byte types (`u8`, `i8`)
endianness is meaningless - use the plain name.

### Unsigned

| Native | Big-endian | Little-endian | Rust type |
|--------|------------|---------------|-----------|
| `u8` | - | - | `u8` |
| `u16` | `be_u16` | `le_u16` | `u16` |
| `u32` | `be_u32` | `le_u32` | `u32` |
| `u64` | `be_u64` | `le_u64` | `u64` |
| `u128` | `be_u128` | `le_u128` | `u128` |

### Signed

| Native | Big-endian | Little-endian | Rust type |
|--------|------------|---------------|-----------|
| `i8` | - | - | `i8` |
| `i16` | `be_i16` | `le_i16` | `i16` |
| `i32` | `be_i32` | `le_i32` | `i32` |
| `i64` | `be_i64` | `le_i64` | `i64` |
| `i128` | `be_i128` | `le_i128` | `i128` |

### Floating-point

| Native | Big-endian | Little-endian | Rust type |
|--------|------------|---------------|-----------|
| `f32` | `be_f32` | `le_f32` | `f32` |
| `f64` | `be_f64` | `le_f64` | `f64` |

### Length helpers

The container `len(...)` decoder produces a `usize` and its encoder takes one:

| Decoder | Encoder | Purpose |
|---------|---------|---------|
| `u8_as_usize` | `u8_from_usize` | 1-byte length |
| `be_u16_as_usize` | `be_u16_from_usize` | 2-byte BE length |
| `le_u16_as_usize` | `le_u16_from_usize` | 2-byte LE length |
| `be_u32_as_usize` | `be_u32_from_usize` | 4-byte BE length |
| `le_u32_as_usize` | `le_u32_from_usize` | 4-byte LE length |
| `be_u64_as_usize` | `be_u64_from_usize` | 8-byte BE length |
| `le_u64_as_usize` | `le_u64_from_usize` | 8-byte LE length |
| `be_u128_as_usize` | `be_u128_from_usize` | 16-byte BE length |
| `le_u128_as_usize` | `le_u128_from_usize` | 16-byte LE length |

### Variable-length (lengthed) decoders

Each numeric type also has a `_lengthed` variant that reads the value's `len`
bytes, zero-extending a shorter value and keeping a longer one's low-order
bytes:

```rust,ignore
#[klv(key = 0x01, size(lengthed), dec = decb::be_u32_lengthed)]
value: u32,
```

Available for all types and endiannesses, e.g. `be_u16_lengthed`,
`le_u32_lengthed`, `be_u64_lengthed`, etc.

## BER (`tinyklv::dec::ber` / `tinyklv::enc::ber`)

Variable-width encodings from ITU-T X.690.

| Decoder | Encoder | Purpose |
|---------|---------|---------|
| `ber_length` | `ber_length` | BER length (container `len(...)`) |
| `ber_oid` | `ber_oid` | BER-OID (container `key(...)`) |

`ber_oid` reads `81 00` as 128; `ber_length` rejects `80`, the indefinite
form. Non-canonical forms: [Limits](./limits.md#non-canonical-ber).

## Strings (`tinyklv::dec::string` / `tinyklv::enc::string`)

Variable-length string codecs. Use with `size(lengthed)` on the field.

| Decoder | Encoder | Produces |
|---------|---------|----------|
| `to_string_utf8` | `from_string_utf8` | `String` (lossy) |
| `to_string_utf8_strict` | - | `String` (returns `Err` on invalid UTF-8) |
| `to_string_utf16_be` | `from_string_utf16_be` | `String` |
| `to_string_utf16_le` | `from_string_utf16_le` | `String` |
| `to_string_ascii` ⚑ | `from_string_ascii` ⚑ | `String` |

⚑ = gated behind the `ascii` crate feature (`features = ["ascii"]` or `features = ["full"]`).

The UTF-16 codecs assume the value body is a raw UTF-16 code-unit
stream with no BOM.

## Utility macros

`tinyklv` exports helper macros that expand to closures matching the
decoder / encoder contract, usable **inline** in `dec = ...`, `enc = ...` and
`default(..)`, as [10 - Macros](../tutorial/fundamentals/10-macros.md) shows.

### Decode macros

| Macro | Expands to | Purpose |
|-------|-----------|---------|
| `scale!(parser, precision, factor)` | `\|input\| -> Result<precision>` | Parse via `parser`, cast to `precision`, multiply by `factor` |
| `cast!(parser, precision)` | `\|input\| -> Result<precision>` | Parse via `parser`, cast to `precision` via `as` |

### Encode macros

| Macro | Expands to | Purpose |
|-------|-----------|---------|
| `scale_enc!(encoder, precision, data, factor)` | `\|val: &precision, out: &mut Vec<u8>\|` | Divide by `factor`, cast to `data`, encode |
| `scale_offset_enc!(encoder, precision, data, factor, offset)` | `\|val: &precision, out: &mut Vec<u8>\|` | Subtract `offset`, divide by `factor`, cast to `data`, encode |
| `cast_enc!(encoder, precision, data)` | `\|val: &precision, out: &mut Vec<u8>\|` | Cast `precision` to `data` via `as`, encode |

### Feature-gated (`chrono`)

| Macro | Expands to | Purpose |
|-------|-----------|---------|
| `as_date!(str_parser, format, len)` | `\|input\| -> Result<NaiveDate>` | Parse `len` bytes as string, then as date |
| `as_time!(str_parser, format, len)` | `\|input\| -> Result<NaiveTime>` | Parse `len` bytes as string, then as time |
| `as_datetime!(str_parser, format, len)` | `\|input\| -> Result<NaiveDateTime>` | Parse `len` bytes as string, then as datetime |
