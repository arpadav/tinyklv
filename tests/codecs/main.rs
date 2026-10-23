//! Codec unit-test harness
//!
//! Declares the codec sub-modules so `cargo test` picks them up
//!
//! Each test sub-module is a focused file of tests for one codec family:
//! `ascii` and `string_ascii` exercise the ASCII string codecs,
//! `ber` covers BER variable-length encoding, and
//! the `binary_*` modules cover fixed-width and lengthed binary codecs. `common` is the
//! test-support library the suites share
//!
//! Author: aav
// --------------------------------------------------
// mods
// --------------------------------------------------
#[cfg(feature = "ascii")]
mod ascii;
#[cfg(feature = "ber")]
mod ber;
mod binary_be;
mod binary_le;
mod binary_lengthed;
mod binary_usize;
#[path = "../common/mod.rs"]
#[expect(dead_code, reason = "a suite uses only part of the shared library")]
mod common;
#[cfg(feature = "ascii")]
mod string_ascii;
mod string_utf16;
mod string_utf8;
