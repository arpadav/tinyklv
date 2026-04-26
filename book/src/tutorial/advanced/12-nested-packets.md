# Tutorial 12 - Nested packets

Real protocols carry sub-structures: a heartbeat may embed a GPS fix, an
engine health block, or radio telemetry. Each sub-structure is itself a KLV
body with its own key/length/value triples, occupying the value region of
one field in the outer frame.

Similar to how custom structs can `impl DecodeValue` in order to call 
`T::decode_value`, well, you can have multiple structs `#[derive(Klv)]`
and use them like so:

```rust,ignore
#[derive(Klv)]
#[klv(..)]
struct GpsFix {
    #[klv(key = 0x02)]
    val: u8,
}

#[derive(Klv)]
#[klv(..)]
struct Heartbeat {
    #[klv(
        key = 0x02,
        dec = GpsFix::decode_value,
        enc = GpsFix::encode_value,
    )]
    gps: GpsFix,
}
```

Because `#[derive(Klv)]` on the inner type generates `impl DecodeValue` and
`impl EncodeValue`, both methods exist and compose cleanly.

The inner type is sentinel-less - it never appears as a top-level frame. It
only shows up as the value region of a field in the outer struct. It can
still be decoded in isolation (useful for unit tests of the inner data
format), as the example demonstrates at the end.

## Example

Run this example: `cargo run --example book_12_nested_packets`

```rust
{{#include ../../../../examples/book_12_nested_packets.rs}}
```

- Nested KLV is encoded explicitly; there is no implicit recursion.
- Inner types derive `Klv` with no sentinel and get `DecodeValue`/`EncodeValue` for free.

**Next:** [13 - Repeated extraction](./13-repeated-extraction.md)
