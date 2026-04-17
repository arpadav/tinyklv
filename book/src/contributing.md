# Contributing

Issues and pull requests are welcome at
<https://github.com/arpadav/tinyklv>.

## Local development

```sh
git clone https://github.com/arpadav/tinyklv
cd tinyklv

# unit + integration + proptests + doctests
cargo test --all

# lint: zero warnings gate
cargo clippy --all-targets -- -D warnings

# examples build
cargo build --examples

# the book
mdbook build book
mdbook test book
```

## Pull request checklist

- [ ] `cargo fmt --all` has been run.
- [ ] `cargo clippy --all-targets -- -D warnings` passes with zero
      warnings.
- [ ] `cargo test --all` passes.
- [ ] New public items carry doc comments (the crate runs with
      `#![deny(missing_docs)]`).
- [ ] Any new attribute is documented in
      [`reference/attributes.md`](./reference/attributes.md).
- [ ] Any new trait method is documented in
      [`reference/traits.md`](./reference/traits.md).

## Code style

- Format with rustfmt.
- No `Box::leak` in proc-macro paths - attribute parsing happens at
  every build.
- Parser errors use `winnow::error::ParserError::from_input(input)`; do
  not rebuild `ContextError` from scratch.

## Questions

Open an issue with the `question` label if the documentation is
unclear, or start a GitHub Discussion if the scope is broader than a
single fix.
