//! Property-based roundtrip test harness
//!
//! Declares the `proptest` sub-modules so `cargo test` picks them up
//! Each sub-module targets a codec family: `ber` covers BER length and OID,
//! `ascii` covers ASCII numeric codecs, `binary_be` / `binary_le` cover
//! fixed-width big- and little-endian binary codecs, `binary_lengthed`
//! covers lengthed binary codecs, and `strings` covers UTF-8 / UTF-16
//! string codec roundtrips
//!
//! Author: aav
// --------------------------------------------------
// mods
// --------------------------------------------------
#[cfg(feature = "ascii")]
mod ascii;
mod ber;
mod binary_be;
mod binary_le;
mod binary_lengthed;
mod strings;
