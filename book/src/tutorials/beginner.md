# Beginner

Five short walk-throughs covering the ground floor of `#[derive(Klv)]`.
Each chapter pairs a single runnable example file with the minimum prose
needed to make the code obvious.

- [01 - Hello World](./01-hello-world.md) - two fields, full roundtrip.
- [02 - Custom Key Types](./02-custom-key-types.md) - swap the key codec.
- [03 - Strings and Primitives](./03-strings-and-types.md) - mixed-type packets.
- [04 - Optional Fields](./04-optional-fields.md) - `Option<T>` and absent keys.
- [05 - Basic Roundtrip](./05-basic-roundtrip.md) - `encode_value` vs `encode_frame`.

Every chapter mirrors a file in [`examples/`](https://github.com/arpadav/tinyklv/tree/main/examples).
Run any with `cargo run --example <name>`.
