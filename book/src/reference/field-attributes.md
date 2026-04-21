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
    key = 0x03,
    enc = MyStruct::encode_value,
)]
custom: MyStruct,
```

Encoders by default expect `fn(&T) -> Vec<u8>`. For example, above
`MyStruct::encode_value` will have the signature `fn(&self) -> Vec<u8>`.

However, the `&` sigil indicates that a coersion must be performed to allow
the encoders of `fn(T) -> Vec<u8>` to accept reference values of `&T`, or others.
In this example, `tinyklv::binary::enc::*` function signatures all take `fn(T) -> Vec<u8>`,
so the `&` is used to allow then to accept `&T`.

This is useful via the `tinyklv::EncodeAs` trait. For example, owned values such
as `String` who use encoders with `&str` function signatures are allowed, since
`tinyklv::EncodeAs` implements `String` with its "smart ref" as `&str`. This is the same
for smart-pointers types, slices, and more.

## `varlen = true`

```rust,ignore
#[klv(key = 0x07, varlen = true, dec = decb::to_string_utf8)]
station_id: String,
```

Selects the length-taking decoder shape. Instead of `fn(&mut S) -> Result<T>`,
the derive calls `dec_fn(len)(input)` where `dec_fn` matches
`fn(len: usize) -> impl Fn(&mut S) -> Result<T>`. Canonical use: UTF-8
strings and other length-prefixed payloads.

## `init = <expr>`

```rust,ignore
#[klv(key = 0x05, init = 0)]
uptime_s: u32,
```

Fallback value used when the key is absent in the decoded stream. Without
a default, an absent key raises a parse error. `init` is the companion form
for initialiser expressions that need to run once per decode call.

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
| `key = <lit>` | [01 - First packet](../tutorial/01-first-packet.md) |
| `dec = <path>` | [01 - First packet](../tutorial/01-first-packet.md) |
| `enc = <path>` | [09 - Encoding & the `&` sigil](../tutorial/09-encode-sigil.md) |
| `enc = &<path>` | [09 - Encoding & the `&` sigil](../tutorial/09-encode-sigil.md) |
| `varlen = true` | [07 - Variable-length fields](../tutorial/07-varlen.md) |
| `init = <expr>` | [10 - Optional fields & init](../tutorial/10-init-fallback.md) |
| `latebind = <path>` | [08 - Latebind transforms](../tutorial/08-latebind.md) |
| `latebind = &mut <path>` | [08 - Latebind transforms](../tutorial/08-latebind.md) |
