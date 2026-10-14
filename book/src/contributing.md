# Contributing

Issues and pull requests are welcome at
<https://github.com/arpadav/tinyklv>.

## Local development

```sh
git clone https://github.com/arpadav/tinyklv
cd tinyklv

# unit + integration + ui + proptests + doctests (ui snapshots: rustc 1.95.0)
cargo +1.95.0 test --workspace --all-features

# lint: zero warnings gate
cargo clippy --workspace --all-targets --all-features -- -D warnings

# examples build + run
bash examples/run-examples.sh

# the book
mdbook build book
```

## Pull request checklist

- [ ] `cargo fmt --all` has been run, and the clippy and test commands above
      pass.
- [ ] New public items carry doc comments.
- [ ] Any new container attribute is documented in [`reference/container-attributes.md`](./reference/container-attributes.md) and any new field attribute in [`reference/field-attributes.md`](./reference/field-attributes.md).
- [ ] Any new trait method is documented in
      [`reference/traits.md`](./reference/traits.md).

## Code style

- Format with rustfmt.
- Parser errors use `winnow::error::ParserError::from_input(input)`; do
  not rebuild `ContextError` from scratch.
- Idiomatic Rust

## Questions

Open an issue with the `question` label if the documentation is
unclear, or start a GitHub Discussion if the scope is broader than a
single fix.
