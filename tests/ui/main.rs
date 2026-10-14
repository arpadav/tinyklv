//! `#[derive(Klv)]` inputs that must not compile, each pinned by message and location
//!
//! Every `fail/<group>/*.rs` case, or `fail/<group>/<topic>/*.rs` in a group split by topic, has
//! a committed `.stderr` snapshot of its diagnostic. A group holds the cases of one stage of the
//! derive: `framing/` which of `key`/`len`/`keylen` a container declares and what they conflict
//! with, its `key_len/` the key + len pair and `keylen/` the fused codec, `header/` the form and
//! fields inside one of them, `size/` a field's or a `key`/`len` slot's `size(..)`, `skip/` a
//! field's `skip_if_default` / `skip_if`, `break_on/` a field's `break_on`, `default/` a container
//! `default(typ = ..)` and the `size(..)` it fills into a field, `unknown/` a container's
//! `unknown(..)` and the `#[klv(unknown)]` field that keeps its frames, `oneof/` a field's
//! `key = oneof(..)` and the settings beside it, `key/` a key that two fields share, `flag/` a
//! container flag such as `strict` written twice, `traits/` a sealed trait a user may not
//! implement or a generated partial they may not build, and `codec/`
//! the header, field and unknown-frame codecs a container names: a literal in place of a codec's
//! path, a signature that does not fit the code the derive generates around it, a
//! `#[klv(unknown)]` field type it cannot push a frame into, or a missing encoder that leaves
//! `EncodeValue` unimplemented, its `header/` the frame header codecs, `field/` a field's,
//! `unknown/` the unknown frames' and `break_on/` a struct or field `break_on` fn. Regenerate the
//! snapshots with `TRYBUILD=overwrite cargo +1.95.0 test --test ui`
//!
//! Author: aav

/// Every case under `fail/` fails to compile with exactly its `.stderr`
#[test]
fn ui() {
    trybuild::TestCases::new().compile_fail("tests/ui/fail/**/*.rs");
}
