# Tutorial 11 - Nested packets

Real protocols carry sub-structures: a heartbeat may embed a GPS fix, an
engine health block, or radio telemetry. Each sub-structure is itself a KLV
body with its own key/length/value triples, occupying the value region of
one field in the outer frame.

Tinyklv does not auto-unwrap nested KLV. The outer field is wired to the
inner type's methods **explicitly**:

```rust,ignore
#[klv(
    key = 0x04,
    dec = GpsFix::decode_value,
    enc = GpsFix::encode_value,
)]
gps: GpsFix,
```

Because `#[derive(Klv)]` on the inner type generates `impl DecodeValue` and
`impl EncodeValue`, both methods exist and compose cleanly. No sigil on
`enc`: derive-generated `encode_value` takes `&self`, which is already the
shape the outer derive expects. (Full story on why:
[Sigil coercion and `EncodeAs`](../reference/sigil-coercion.md).)

The inner type is sentinel-less - it never appears as a top-level frame. It
only shows up as the value region of a field in the outer struct. It can
still be decoded in isolation (useful for unit tests of the inner wire
format), as the example demonstrates at the end.

```rust,no_run
{{#include ../../../examples/book_11_nested_packets.rs}}
```

- Nested KLV is wired explicitly; there is no implicit recursion.
- Inner types derive `Klv` with no sentinel and get `DecodeValue`/`EncodeValue` for free.
- No `&` sigil on the outer field - derive-generated methods already take `&self`.

**Next:** [12 - Repeated extraction](./12-repeated-extraction.md)
