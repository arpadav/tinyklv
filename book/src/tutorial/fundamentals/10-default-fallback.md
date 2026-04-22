# Tutorial 10 - Optional fields, `default`, and fallback impls

Protocols evolve. A v1 transmitter emits three keys; a v2 transmitter adds
two more. If the decoder errors on every missing key, v1 frames become
unintelligible to a v2 receiver - and the whole point of KLV is that
unknown senders should still produce readable bytes.

Three attributes handle the asymmetry:

### `Option<T>` fields

The derive emits `None` when the key is absent from the stream. Encoding a
`None` emits no KLV triple. Use when the caller genuinely wants "field was
missing" distinguishable from "field was present with some value".

### `default` / `default = expr`

The derive supplies a concrete fallback when the key is absent. The field
type stays `T`, not `Option<T>`.

- Bare `default` calls `<T as ::core::default::Default>::default()` -
  handy when the type already has a sensible zero value.
- `default = expr` inlines the given expression, letting you supply any
  compile-time fallback.

### `fallback_impls` (container-level)

Any field lacking explicit `enc`/`dec` and not matched by a container
`default(..)` falls back to the `EncodeValue` / `DecodeValue` trait impls
on the field type.

## `Option<T>` and `default` example

A short packet with three keys: a required `sequence`, an optional
`signal_dbm`, and a `battery_pct` that falls back to `100` when absent.
The thin stream below is hand-rolled to show the wire grammar - just
`key, len, value` triples. Decoding it produces `None` and `100` for the
two missing keys; no error, no panic.

Run this example: `cargo run --example book_10_default`

```rust,no_run
{{#include ../../../../examples/book_10_default.rs}}
```

- `Option<T>` surfaces "key was absent" explicitly to the caller.
- `default = expr` supplies a concrete fallback; the field type stays `T`.
- Bare `default` (no `=`) calls `Default::default()` on the field type -
  handy when the type already has a sensible zero value.
- Both are field-level and compose with every other `#[klv(...)]` option.

## Fallback Implementation

Sometimes a field's type already knows how to encode/decode itself - a
newtype wrapping a protocol-defined ID, a hand-rolled enum, a nested
derived struct. Spelling out `enc =`/`dec =` on every field that refers
to such a type is noise.

Set the container-level flag `fallback_impls` and the derive will, for any
field without its own `enc`/`dec`, call the field type's
`EncodeValue` / `DecodeValue` impls directly.

Run this example: `cargo run --example book_10_fallback`

```rust,no_run
{{#include ../../../../examples/book_10_fallback.rs}}
```

- Field-level `enc` / `dec` still wins over the fallback.
- Container `default(typ = T, ..)` still wins over the fallback.
- Fallback runs last - it is opt-in and silent unless reached.
- `Option<T>` composes: a missing key stays `None`, a present one defers
  to `T`'s impls.

**Next:** [11 - Nested packets](./11-nested-packets.md)
