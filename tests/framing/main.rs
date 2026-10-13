//! Key + len framing integration-test harness
//!
//! Declares the framing test sub-modules so `cargo test` can discover them. `common` is the
//! test-support library the suites share; the remaining sub-modules each cover one framing
//! feature of a key + len struct, of its fields' frames or of the frames no field declares
//!
//! Author: aav
// --------------------------------------------------
// mods
// --------------------------------------------------
#[path = "../common/mod.rs"]
#[expect(dead_code, reason = "a suite uses only part of the shared library")]
mod common;
mod default_lengthed;
mod default_override;
mod exact;
mod exact_stateful;
mod field_break_on;
mod implicit;
mod oneof;
mod skip;
mod unknown;
