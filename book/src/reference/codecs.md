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

### Signed

| Native | Big-endian | Little-endian | Rust type |
|--------|------------|---------------|-----------|
| `i8` | - | - | `i8` |
| `i16` | `be_i16` | `le_i16` | `i16` |
| `i32` | `be_i32` | `le_i32` | `i32` |
| `i64` | `be_i64` | `le_i64` | `i64` |

### Floating-point

| Native | Big-endian | Little-endian | Rust type |
|--------|------------|---------------|-----------|
| `f32` | `be_f32` | `le_f32` | `f32` |
| `f64` | `be_f64` | `le_f64` | `f64` |

### Length helpers

For the container `len(...)` attribute you need a decoder that
produces `usize` and an encoder that accepts `usize`:

| Decoder | Encoder | Purpose |
|---------|---------|---------|
| `u8_as_usize` | `u8_from_usize` | 1-byte length |
| `be_u16_as_usize` | `be_u16_from_usize` | 2-byte BE length |
| `le_u16_as_usize` | `le_u16_from_usize` | 2-byte LE length |
| `be_u32_as_usize` | `be_u32_from_usize` | 4-byte BE length |
| `le_u32_as_usize` | `le_u32_from_usize` | 4-byte LE length |
| `be_u64_as_usize` | `be_u64_from_usize` | 8-byte BE length |
| `le_u64_as_usize` | `le_u64_from_usize` | 8-byte LE length |

### Variable-length (lengthed) decoders

Each numeric type also has a `_lengthed` variant that reads a
variable number of bytes (up to the type width) and zero-extends:

```rust,ignore
#[klv(key = 0x01, varlen = true, dec = decb::be_u32_lengthed)]
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

Edge cases:
- Empty input -> decoder returns `Err`.
- Single MSB-set byte with no terminator -> decoder returns `Err`.
- `[0x00]` decodes as `0`; `[0x81, 0x00]` decodes as `128`.

## Strings (`tinyklv::dec::string` / `tinyklv::enc::string`)

Variable-length string codecs. Use with `varlen = true` on the field.

| Decoder | Encoder | Produces |
|---------|---------|----------|
| `to_string_utf8` | `from_string_utf8` | `String` (lossy) |
| `to_string_utf8_strict` | - | `String` (returns `Err` on invalid UTF-8) |
| `to_string_utf16_be` | `from_string_utf16_be` | `String` |
| `to_string_utf16_le` | `from_string_utf16_le` | `String` |
| `to_string_ascii` ⚑ | `from_string_ascii` ⚑ | `String` |

⚑ = gated behind the `ascii` crate feature (`features = ["ascii"]` or `features = ["full"]`).

The UTF-16 codecs assume the value body is a raw UTF-16 code-unit
stream with no BOM. Choose `_be` or `_le` to match your data format.

## Utility macros

`tinyklv` exports helper macros that expand to closures matching the
decoder / encoder contract. They can be used **inline** in `dec = ...`,
`enc = ...`, and `default(typ = T, dec = ..., enc = ...)` attributes -
any macro that expands to a function or closure with the right
signature works.

### Decode macros

| Macro | Expands to | Purpose |
|-------|-----------|---------|
| `scale!(parser, precision, factor)` | `\|input\| -> Result<precision>` | Parse via `parser`, cast to `precision`, multiply by `factor` |
| `cast!(parser, precision)` | `\|input\| -> Result<precision>` | Parse via `parser`, cast to `precision` via `as` |

### Encode macros

| Macro | Expands to | Purpose |
|-------|-----------|---------|
| `scale_enc!(encoder, precision, data, factor)` | `\|val: &precision\| -> Vec<u8>` | Divide by `factor`, cast to `data`, encode |
| `scale_offset_enc!(encoder, precision, data, factor, offset)` | `\|val: &precision\| -> Vec<u8>` | Subtract `offset`, divide by `factor`, cast to `data`, encode |
| `cast_enc!(encoder, precision, data)` | `\|val: &precision\| -> Vec<u8>` | Cast `precision` to `data` via `as`, encode |

### Inline usage

```rust,ignore
const HEADING_SCALE: f64 = 360.0 / 65535.0;

#[klv(
    key = 0x02,
    dec = tinyklv::scale!(decb::be_u16, f64, HEADING_SCALE),
    enc = tinyklv::scale_enc!(
        encb::be_u16, f64, u16, HEADING_SCALE,
    ),
)]
heading_deg: f64,
```

The `dec = ...` attribute accepts any expression that resolves to
`fn(&mut S) -> tinyklv::Result<T>`. Built-in codecs, named functions,
and macro invocations all satisfy this. Closures do **not** work
inline (proc-macro attribute parsing limitation) - use a named
function instead.

See the full example: `cargo run --example book_05b_macro_decoders`

### Feature-gated (`chrono`)

| Macro | Expands to | Purpose |
|-------|-----------|---------|
| `as_date!(str_parser, format, len)` | `\|input\| -> Result<NaiveDate>` | Parse `len` bytes as string, then as date |
| `as_time!(str_parser, format, len)` | `\|input\| -> Result<NaiveTime>` | Parse `len` bytes as string, then as time |
| `as_datetime!(str_parser, format, len)` | `\|input\| -> Result<NaiveDateTime>` | Parse `len` bytes as string, then as datetime |
