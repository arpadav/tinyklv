//! Fused `#[klv(keylen(..))]` integration-test harness
//!
//! Declares the fused-codec test sub-modules so `cargo test` can discover them. `common` is the
//! test-support library the suites share, `support` the codecs and messages this suite's files
//! share; the remaining sub-modules each cover one slice of the fused codec
//!
//! Author: aav
// --------------------------------------------------
// mods
// --------------------------------------------------
mod break_on;
#[path = "../common/mod.rs"]
#[expect(dead_code, reason = "a suite uses only part of the shared library")]
mod common;
mod const_locals;
mod decode_basic;
mod encode_roundtrip;
mod exact;
mod field_break_on;
mod fieldless;
mod frame_dispatch;
mod frame_progress;
mod framing_rules;
mod inline_error;
mod lengthed;
mod macro_hygiene;
mod oneof;
mod strict;
mod support;
