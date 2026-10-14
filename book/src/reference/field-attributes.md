# Field attributes

Attributes declared on individual struct fields with `#[klv(...)]`.

## `key = <literal>`

```rust,ignore
#[klv(key = 0x02)]
temperature_centideg: u16,
```

The key byte(s) the field is matched on at decode time and emitted at
encode time. Literal types match the codec in the container's
`key(dec = ..., enc = ...)` - usually an integer literal for byte keys.

## `key = oneof(K => Enum::Variant(Type), ..)`

```rust,ignore
#[klv(key = oneof(
    0x07 => Payload::Count(u16),
    0x08 => Payload::Note(String),
))]
payload: Option<Payload>,
```

Routes several keys to one `Option<Enum>` field, or a required plain `Enum`.
An item of key `K` holds a `Type`, coded by the container
`default(typ = Type, ..)` or `trait_fallback`, and is stored as
`Enum::Variant`; the last one wins. Encode writes the set variant's item in the
field's place; for prost's bytes, declare fields in key order, a oneof at its
lowest key. The field takes no `enc`, `dec` or `size(..)`.

## `dec = <path>`

```rust,ignore
#[klv(key = 0x01, dec = decb::u8)]
sequence: u8,
```

Associates the codec pair for this field. `dec` must match
`fn(&mut S) -> tinyklv::Result<T>`, and `enc` `fn(&T, &mut Vec<u8>)` (a
value-taking one takes the `*` sigil below). Either can be omitted when the
container default covers the field type.

## `enc = <optional sigil> <path>`

```rust,ignore
#[klv(
    key = 0x02,
    enc = &encs::from_string_utf8,
)]
name: String,

#[klv(
    key = 0x01,
    enc = *encb::u8,
)]
sequence: u8,

#[klv(
    key = 0x03,
    enc = MyStruct::encode_value,
)]
custom: MyStruct,
```

The two sigils (`&` and `*`) rewrite how the field is handed to the encoder:
see [Sigil coercion & `EncodeAs`](./sigil-coercion.md).

## `size(..)`

```rust,ignore
#[klv(key = 1)]                                      n: u16,    // 01 02 01 02
#[klv(key = 1, size(exact = 2))]                     n: u16,    // 01 02 01 02
#[klv(key = 1, size(lengthed))]                      s: String, // 01 02 68 69
#[klv(key = 1, size(exact = 2, lengthed))]           s: String, // 01 02 68 69
#[klv(key = 1, size(implicit))]                      n: u16,    // 01 01 02
#[klv(key = 1, size(implicit, exact = 2))]           n: u16,    // 01 01 02
#[klv(key = 1, size(implicit, exact = 2, lengthed))] s: String, // 01 68 69
#[klv(key = 1, size(implicit, lengthed))]            s: String, // compile error
```

Four keywords, one fact each: `implicit` writes and reads no length prefix,
`exact = N` makes the value `N` bytes, `lengthed` passes the length to the
decoder, and `reserve = N` adds `N` bytes to the encoder's one up-front
allocation. Only `lengthed` changes the decoder, to
[`fn(len: usize) -> impl Fn(&mut S) -> Result<T>`](./codecs.md#function-signatures).
Under `implicit, exact = N` the encoder must write `N` bytes; a release build
does not check.

Each line above is one row below, under `len(.., size(exact = 1))`, with
`n: u16 = 0x0102` taking `be_u16` and `s: String = "hi"` taking
`to_string_utf8` from a `default(typ = String, .., size(lengthed))`.
`len(size(exact = W))` sizes the slot back-filled after a value of unknown
width; an `exact = N` value's length is written before it ("direct") and may be
longer than `W`. A variable-width `len(..)` stages every value it prefixes. An
`N` wider than `W` bytes can hold is a compile error.

| `implicit` | `exact = N` | `lengthed` | decode | encode |
|---|---|---|---|---|
| | | | read `len`, `take(len)` | key, `len`, value |
| | ✓ | | read `len`, error if `!= N`, `take(N)` | key, `len(N)`, value direct |
| | | ✓ | read `len`, `take(len)`, pass `len` | key, `len`, value |
| | ✓ | ✓ | read `len`, error if `!= N`, `take(N)`, pass `N` | key, `len(N)`, value direct |
| ✓ | | | no `len`, no `take`; decoder stops itself | key, value |
| ✓ | ✓ | | no `len`, `take(N)` | key, value |
| ✓ | ✓ | ✓ | no `len`, `take(N)`, pass `N` | key, value |
| ✓ | | ✓ | **compile error**: no length to pass | — |

`reserve = N` changes no byte on the wire. It combines with any row without
`exact`, and a field's own `exact` or `reserve` replaces its default's:

| field | result |
|---|---|
| `#[klv(key = 0x1A, size(reserve = 64))] id: String` | `1A 02 68 69`, as without `reserve` |
| `#[klv(key = 1, size(exact = 2, reserve = 64))] n: u16` | **compile error**: a known width already reserves itself |
| `#[klv(key = 1, size(reserve = 64))] n: u16`, its default `size(exact = 2)` | reserve only: `01 03 01 02 00` decodes, no length check |

## `default` / `default = <expr>`

```rust,ignore
#[klv(key = 0x05, default)]
uptime_s: u32,

#[klv(key = 0x06, default = 100_u8)]
battery_pct: u8,
```

Fallback value used when the key is absent in the decoded stream. Without
either form, a missing non-`Option` field is an error.

| Form | Emitted fallback |
|------|------------------|
| `default` (no value) | `<FieldType as ::core::default::Default>::default()` |
| `default = <expr>` | the inline expression, evaluated once per decode call |

## `skip_if_default` / `skip_if = <path>`

```rust,ignore
#[klv(key = 0x08, skip_if_default)]
count: u64,

#[klv(key = 0x1A, skip_if = String::is_empty)]
name: String,
```

Leaves the field's item out of the encode: `skip_if_default` when the field
equals its type's `Default::default()`, `skip_if` when the `fn(&T) -> bool`
returns `true`; as in prost, `-0.0` equals `0.0` and is left out. A missing
item then decodes to `Default::default()`, unless the field sets its own
`default`. On an `Option<T>` field the test takes the whole field: `None` is
never written, `Some(0)` is. One form, once, on a field or per type in a
container `default(typ = ..)`; a field's own replaces its default's.

## `latebind = <optional sigil> <path>`

```rust,ignore
#[klv(
    key = 0x06,
    dec = decb::u8,
    latebind = Mode::from_u8,
)]
mode: Mode,

#[klv(
    key = 0x07,
    dec = Coordinate::decode_value,
    latebind = &mut Coordinate::apply_global_z,
)]
position: Coordinate,
```

Runs after the field decoder.

Without the `&mut` sigil, the signature is `Fn(T) -> U`. The decoder produces
`T`, `latebind` promotes it to the field's declared type `U`.

With the `&mut` sigil, the signature is `Fn(&mut T)`. The field type stays `T`; the
function patches the decoded value in place. 

## `break_on = <path>`

```rust,ignore
#[klv(key = 0x04, break_on = check_status)] // fn(&Status) -> BreakType
status: Status,
```

Runs `fn(&value) -> BreakType` after the field decodes: `Proceed` keeps the
value, `Skip` discards it, `Done` keeps it and ends the decode, and `Abort`
errors with its message (one-shot: field first). Encode writes these fields
last. The container-level
[`break_on`](./container-attributes.md#break_on--key-literal--path) checks
keys.

## `unknown`

```rust,ignore
#[klv(unknown)]
extra: Vec<(u8, Vec<u8>)>,
```

Keeps every item whose key no field declares, as `(key, value)` in wire
order, decoded by the container
[`unknown(dec = ..)`](./container-attributes.md#unknowndec---enc--). Its
`enc` writes them before any `break_on` field, after the rest. A malformed kept
item is an error naming this field. One per struct, beside a container
`unknown(..)`, with no other setting.

## Cross-reference

| Attribute | First introduced in |
|-----------|--------------------|
| `key = <lit>` | [01 - First packet](../tutorial/fundamentals/01-first-packet.md) |
| `dec = <path>` | [01 - First packet](../tutorial/fundamentals/01-first-packet.md) |
| `enc = <path>` | [09 - Encoding & sigils](../tutorial/fundamentals/09-encode-sigil.md) |
| `enc = &<path>` | [09 - Encoding & sigils](../tutorial/fundamentals/09-encode-sigil.md) |
| `enc = *<path>` | [09 - Encoding & sigils](../tutorial/fundamentals/09-encode-sigil.md) |
| `size(lengthed)` | [07 - Variable-length fields](../tutorial/fundamentals/07-val-lengths.md) |
| `size(implicit)` | [17 - Values with no length](../tutorial/advanced/17-implicit.md) |
| `default` | [11 - Optional fields & default](../tutorial/fundamentals/11-default-fallback.md) |
| `default = <expr>` | [11 - Optional fields & default](../tutorial/fundamentals/11-default-fallback.md) |
| `latebind = <path>` | [08 - Latebind transforms](../tutorial/fundamentals/08-latebind.md) |
| `latebind = &mut <path>` | [08 - Latebind transforms](../tutorial/fundamentals/08-latebind.md) |
| `unknown` | [19 - Unknown items](../tutorial/advanced/19-unknown.md) |
