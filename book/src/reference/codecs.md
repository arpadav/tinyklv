# Codecs

Built-in codec paths shipped with `tinyklv`. Every path listed is a
drop-in value for `dec = ...`, `enc = ...`, `key(dec = ..., enc = ...)`,
or `len(dec = ..., enc = ...)`.

## Binary (`tinyklv::dec::binary` / `tinyklv::enc::binary`)

Fixed-width big-endian and little-endian integer codecs.

### Unsigned

| Decoder | Encoder | Rust type |
|---------|---------|-----------|
| `be_u8` | `u8` | `u8` |
| `be_u16`, `le_u16` | `be_u16`, `le_u16` | `u16` |
| `be_u32`, `le_u32` | `be_u32`, `le_u32` | `u32` |
| `be_u64`, `le_u64` | `be_u64`, `le_u64` | `u64` |

### Signed

| Decoder | Encoder | Rust type |
|---------|---------|-----------|
| `be_i8` | `i8` | `i8` |
| `be_i16`, `le_i16` | `be_i16`, `le_i16` | `i16` |
| `be_i32`, `le_i32` | `be_i32`, `le_i32` | `i32` |
| `be_i64`, `le_i64` | `be_i64`, `le_i64` | `i64` |

### Floating-point

| Decoder | Encoder | Rust type |
|---------|---------|-----------|
| `be_f32`, `le_f32` | `be_f32`, `le_f32` | `f32` |
| `be_f64`, `le_f64` | `be_f64`, `le_f64` | `f64` |

### Length helpers

For the container `len(...)` attribute you need a decoder that
produces `usize`:

| Decoder | Encoder | Purpose |
|---------|---------|---------|
| `be_u8_as_usize` | `u8_from_usize` | 1-byte length |
| `be_u16_as_usize` | `be_u16_from_usize` | 2-byte BE length |

## BER (`tinyklv::dec::ber` / `tinyklv::enc::ber`)

Variable-width encodings from ITU-T X.690.

| Decoder | Encoder | Purpose |
|---------|---------|---------|
| `ber_length_decode` | `ber_length_encode` | BER length (container `len(...)`) |
| `ber_oid` | `ber_oid` | BER-OID (container `key(...)`) |

Edge cases:
- Empty input → decoder returns `Err`.
- Single MSB-set byte with no terminator → decoder returns `Err`.
- `[0x00]` decodes as `0`; `[0x81, 0x00]` decodes as `128`.

## Strings (`tinyklv::dec::binary` / `tinyklv::enc::string`)

Variable-length string codecs. Use with `var = true` on the field.

| Decoder | Encoder | Produces |
|---------|---------|----------|
| `to_string_utf8` | `from_string_utf8` | `String` |
| `to_string_utf16_be` | `from_string_utf16_be` | `String` |
| `to_string_utf16_le` | `from_string_utf16_le` | `String` |
| `to_string_ascii` | `from_string_ascii` | `String` |

The UTF-16 codecs assume the value body is a raw UTF-16 code-unit
stream with no BOM. Choose `_be` or `_le` to match your wire format.
