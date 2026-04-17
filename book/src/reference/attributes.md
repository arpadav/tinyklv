# Derive Attributes

Every attribute recognised by `#[derive(Klv)]`. Unknown attribute keys
raise a compile error at proc-macro expansion time.

## Container-level `#[klv(...)]`

Applied to the struct itself.

| Attribute | Purpose |
|-----------|---------|
| `stream = <ty>` | Input stream type (any `winnow::Stream`); typically `&[u8]` |
| `sentinel = b"\x..."` | Magic bytes marking a frame start; enables `decode_frame` / `SeekSentinel` |
| `key(dec = path, enc = path)` | Codec pair for the key field |
| `len(dec = path, enc = path)` | Codec pair for the length field (decoder must produce `usize`) |
| `default(typ = T, dec = path, enc = path)` | Default codec pair for every field of type `T` |
| `debug` | Emit generated impl blocks at compile time (for debugging) |
| `deny_unknown_keys` | Error on unrecognized keys instead of skipping |
| `allow_unimplemented_encode` | Skip generating `EncodeValue`/`EncodeFrame` |
| `allow_unimplemented_decode` | Skip generating `DecodeValue`/`DecodeFrame` |

## Field-level `#[klv(...)]`

Applied to a struct field.

| Attribute | Purpose |
|-----------|---------|
| `key = <expr>` | Key value for this field (matched at decode time) |
| `dec = path` | Decoder function `fn(&mut S) -> winnow::Result<T>` |
| `enc = path` | Encoder function `fn(&T) -> Vec<u8>` - takes `&T` by reference (deref coercion handles `&String → &str`, `&Vec<u8> → &[u8]`) |
| `enc = &path` | [`EncodeAs`](#encoder-dispatch-sigils)-dispatched encoder: primitives pass by value, `String → &str`, `Vec<T> → &[T]`, smart pointers → inner ref |
| `var = true` | Field has a variable-width value (length prefix is authoritative) |
| `default = expr` | Value used if the key is absent on decode |
| `sentinel = b"..."` | Per-field sentinel for nested framed fields |
| `stream = <ty>` | Per-field stream override |
| `break` | Break repeated decode when this condition fires |
| `repeated` | Decode this field as a `Vec<T>` of repeated inner frames |

## Codec path conventions

Any path you provide to `dec =` / `enc =` / `key(...)` / `len(...)` is
a Rust path expression. All of these are legal:

```rust,no_run,ignore
dec = tinyklv::dec::binary::be_u16           // crate-qualified
dec = crate::my_codecs::scaled_temperature    // project-local
dec = MyType::decode_value                    // method on another derive
```

The derive does not validate that the path resolves until compile
time - typos surface as Rust compile errors on the generated impl.

## Encoder dispatch sigils

Rust's `Fn` traits are invariant over argument types - the macro cannot
auto-detect whether your encoder takes `&T` or owned `T` at the call
site. Instead, the `enc =` attribute accepts an optional leading `&`
sigil that routes the field value through the
[`EncodeAs`](https://docs.rs/tinyklv/latest/tinyklv/traits/trait.EncodeAs.html)
trait, which dispatches to the zero-cost borrowed form for each type:

| Syntax | Expanded call (required field) | Expanded call (`Option<T>`) | Required signature |
|--------|-------------------------------|------------------------------|--------------------|
| `enc = func` | `func(&self.field)` | `func(__val)` | `Fn(&T) -> O` |
| `enc = &func` | `func(EncodeAs::encode_as(&self.field))` | `func(EncodeAs::encode_as(__val))` | `Fn(<T as EncodeAs>::Borrowed<'_>) -> O` |

### When to use each

**No sigil** - your encoder takes `&T` directly. Rust's deref coercion
already turns `&String → &str` and `&Vec<u8> → &[u8]` at the call site,
so most encoders (including method forms like `MyType::encode_value`)
need no sigil.

**`&` sigil** - route through `EncodeAs`. This covers the shapes where
no-sigil `&T` is wrong:

| Field type | `EncodeAs::Borrowed` | Why |
|------------|---------------------|-----|
| `u8..u128`, `i8..i128`, `usize`, `isize`, `f32`, `f64`, `bool`, `char` | `Self` (by value) | primitives are `Copy`; no clone, no alloc |
| `String` | `&str` | encoder wants `&str`, not `&String` |
| `Vec<T>` | `&[T]` | encoder wants `&[T]`, not `&Vec<T>` |
| `Cow<'_, str>` / `Cow<'_, [T]>` | `&str` / `&[T]` | same |
| `Box<T>`, `Rc<T>`, `Arc<T>` | `&T` | unwrap the pointer |

Custom types are opt-in: implement `EncodeAs` on your type if you want
it usable with `&` - for a `Copy` enum/struct, a one-liner returning
`*self` is typical.

### Example

```rust,ignore
#[derive(Klv)]
#[klv(/* ... */)]
struct Telemetry {
    // be_u16: fn(u16) -> Vec<u8> - primitive passed by value via EncodeAs
    #[klv(key = 0x01, dec = ..., enc = &tinyklv::enc::binary::be_u16)]
    altitude: u16,

    // from_string_utf8: fn(&str) -> Vec<u8> - String → &str via EncodeAs
    #[klv(key = 0x02, dec = ..., enc = &tinyklv::enc::string::from_string_utf8)]
    name: String,

    // MyType::encode_value: fn(&Self) -> Vec<u8> - already &T, no sigil
    #[klv(key = 0x03, dec = ..., enc = MyType::encode_value)]
    custom: MyType,
}
```

### Limitations

- Sigils only apply to field-level `enc =`. Container-level
  `key(enc = ...)` and `len(enc = ...)` encoders do not accept sigils -
  their call shape has no ambiguity.
- The `dec =` side has no sigils: decoders always take
  `fn(&mut S) -> winnow::Result<T>` and return owned `T`.
- If your encoder expression itself starts with `&` (e.g. a
  closure body), the sigil lookahead will consume it. Wrap such
  expressions in parentheses or assign to a named `fn` to avoid this.
