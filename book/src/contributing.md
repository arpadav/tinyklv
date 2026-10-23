# Contributing

Issues and pull requests are welcome at <https://github.com/arpadav/tinyklv>.

## Checks

Run these before you open a pull request. CI runs the first four on every pull request:

```sh
git clone https://github.com/arpadav/tinyklv
cd tinyklv

cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features
cargo +1.95.0 test --workspace --all-features   # every test, and every example
bash examples/run-examples.sh                   # every example, with its output
mdbook build book                               # this book
```

The ui tests check the compiler's messages, which change between Rust versions, so they match 1.95.0's. On another toolchain, skip them:

```sh
cargo test --workspace --all-features -- --exact --skip ui
```

## Pull requests

- format with rustfmt, and pass the checks above
- give every new public item a doc comment
- document a new attribute in [Container attributes](./reference/container-attributes.md) or [Field attributes](./reference/field-attributes.md), and a new trait or method in [Traits](./reference/traits.md)
- assert every byte this book shows in an example under `examples/book/`, registered in `Cargo.toml` as a test

## Code style

- clippy's pedantic lints are errors
- build a decoder's error from its input, with winnow's `ParserError::from_input`, not from an empty `ContextError`
- write idiomatic Rust

## Questions

Open an issue with the `question` label if this book is unclear, or start a GitHub Discussion for anything broader than one fix.
