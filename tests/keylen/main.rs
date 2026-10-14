//! Fused `#[klv(keylen(..))]` integration-test harness
//!
//! Declares the fused-codec test sub-modules so `cargo test` can discover them. `common` is the
//! test-support library the suites share, `support` the codecs and messages this suite's files
//! share; the remaining sub-modules each cover one slice of the fused codec, grouped by topic:
//! `framing` the header's own framing of each frame and its errors, `size`, `field` and
//! `container` the framing features `tests/framing` covers for a key + len struct, and `hygiene`
//! the generated code under a user's constants, macros and traits
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
    mod fieldless;
    mod strict;
}
mod field {
    mod field_break_on;
    mod oneof;
}
mod framing {
    mod decode_basic;
    mod encode_roundtrip;
    mod frame_dispatch;
    mod frame_progress;
    mod framing_rules;
    mod inline_error;
}
mod hygiene {
    mod const_locals;
    mod macro_hygiene;
    mod prelude_free;
}
mod size {
    mod exact;
    mod lengthed;
}
mod support;
