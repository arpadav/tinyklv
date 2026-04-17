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
| `enc = path` | Encoder function `fn(&T) -> Vec<u8>` |
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
