//! Codec unit-test harness
//!
//! Declares the codec sub-modules so `cargo test` picks them up
//!
//! Each test sub-module is a focused file of tests for one codec family:
//! `ascii` and `string_ascii` exercise the ASCII string codecs,
//! `ber_length` / `ber_oid` cover BER variable-length encoding, and
//! the `binary_*` modules cover fixed-width and lengthed binary codecs
//!
//! `helpers` holds the shared `ContextErrorExt` queries on a BER error's `Expected` context
//!
//! Author: aav
// --------------------------------------------------
// mods
// --------------------------------------------------
#[cfg(feature = "ascii")]
mod ascii;
mod ber_length;
mod ber_oid;
mod binary_be;
mod binary_le;
mod binary_lengthed;
mod binary_usize;
mod helpers;
#[cfg(feature = "ascii")]
mod string_ascii;
mod string_utf16;
mod string_utf8;
