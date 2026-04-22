# Field attributes

Attributes declared on individual struct fields with `#[klv(...)]`. Each
section gives syntax, a short semantic summary, and a minimal snippet.

## `key = <literal>`

```rust,ignore
#[klv(key = 0x02)]
temperature_centideg: u16,
```

The key byte(s) the field is matched on at decode time and emitted at
encode time. Literal types match the codec wired in the container's
`key(dec = ..., enc = ...)` - usually an integer literal for byte keys.

## `dec = <path>`

```rust,ignore
#[klv(key = 0x01, dec = decb::u8)]
sequence: u8,
```

Wires the codec pair for this field. `dec` must match
`fn(&mut S) -> tinyklv::Result<T>`, where `tinyklv::Result` is
an alias for `winnow::Result`

`enc` must match `fn(&T) -> Vec<u8>`
(when owned-argument, add the `&` sigil below). Either can be omitted when
the container default covers the field type.

## `enc = <optional sigil> <path>`

```rust,ignore
#[klv(
    key = 0x02,
    enc = &encb::be_u16,
)]
temperature_centideg: u16,

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

Encoders by default expect `fn(&T) -> O`. The two sigils (`&` and `*`)
rewrite how the field is handed to the encoder. See
[Sigil coercion & `EncodeAs`](./sigil-coercion.md) for the full
dispatch semantics, built-in `EncodeAs` implementations, and when to
write your own.

## `varlen = true`

```rust,ignore
#[klv(key = 0x07, varlen = true, dec = decs::to_string_utf8)]
station_id: String,
```

Selects the length-taking decoder shape. Instead of `fn(&mut S) -> Result<T>`,
the derive calls `dec_fn(len)(input)` where `dec_fn` matches
`fn(len: usize) -> impl Fn(&mut S) -> Result<T>`. Canonical use: UTF-8
strings and other length-prefixed payloads.

## `default` / `default = <expr>`

```rust,ignore
#[klv(key = 0x05, default)]
uptime_s: u32,

#[klv(key = 0x06, default = 100_u8)]
battery_pct: u8,
```

Fallback value used when the key is absent in the decoded stream. Without
either form, an absent key raises a parse error.

| Form | Emitted fallback |
|------|------------------|
| `default` (no value) | `<FieldType as ::core::default::Default>::default()` |
| `default = <expr>` | the inline expression, evaluated once per decode call |

Use bare `default` when the field type already has a sensible zero value
(primitives, `Option<T>`, `Vec<T>`, `String`, or any user type with
`#[derive(Default)]`). Use `default = <expr>` when the fallback is a
specific constant, a sentinel, or a non-`Default` value.

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

## Cross-reference

| Attribute | First introduced in |
|-----------|--------------------|
| `key = <lit>` | [01 - First packet](../tutorial/fundamentals/01-first-packet.md) |
| `dec = <path>` | [01 - First packet](../tutorial/fundamentals/01-first-packet.md) |
| `enc = <path>` | [09 - Encoding & sigils](../tutorial/fundamentals/09-encode-sigil.md) |
| `enc = &<path>` | [09 - Encoding & sigils](../tutorial/fundamentals/09-encode-sigil.md) |
| `enc = *<path>` | [09 - Encoding & sigils](../tutorial/fundamentals/09-encode-sigil.md) |
| `varlen = true` | [07 - Variable-length fields](../tutorial/fundamentals/07-val-lengths.md) |
| `default` | [10 - Optional fields & default](../tutorial/fundamentals/10-default-fallback.md) |
| `default = <expr>` | [10 - Optional fields & default](../tutorial/fundamentals/10-default-fallback.md) |
| `latebind = <path>` | [08 - Latebind transforms](../tutorial/fundamentals/08-latebind.md) |
| `latebind = &mut <path>` | [08 - Latebind transforms](../tutorial/fundamentals/08-latebind.md) |
