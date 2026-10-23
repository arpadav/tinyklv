//! Key + len framing integration-test harness
//!
//! Declares the framing test sub-modules so `cargo test` can discover them. `common` is the
//! test-support library the suites share; the remaining sub-modules each cover one framing
//! feature of a key + len struct, grouped by topic: `size` a value's width and the length
//! prefix's own, `field` a field's frame and what a container `default(..)` fills into it,
//! `container` the struct's own settings, the frames no field declares and the error kind of each
//! rejection, and `hygiene` the generated code under the names a user's code puts in its scope
//!
//! Author: aav
// --------------------------------------------------
// mods
// --------------------------------------------------
#[path = "../common/mod.rs"]
#[expect(dead_code, reason = "a suite uses only part of the shared library")]
mod common;
mod container {
    mod break_on;
    mod error_kind;
    mod fieldless;
    mod one_sided;
    mod sentinel;
    mod strict;
    mod terminator;
    mod unknown;
}
mod field {
    mod default_lengthed;
    mod default_override;
    mod field_break_on;
    mod oneof;
    mod skip;
    mod type_fragments;
}
mod hygiene {
    mod const_locals;
    mod glob_variants;
    mod lifetime_name;
    mod no_implicit_prelude;
    mod prelude_free;
}
mod size {
    mod exact;
    mod exact_stateful;
    mod fixed_len_prefix;
    mod implicit;
    mod reserve;
    mod shadowed_binary_module;
    mod shadowed_debug_assert;
    mod shadowed_endian_name;
}
