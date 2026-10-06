//! Edge-case test harness
//!
//! Declares the edge-case sub-modules so `cargo test` picks them up
//! Each module targets a specific failure domain: `boundary_values` verifies
//! roundtrips at numeric and BER type extremes, `duplicate_keys` tests last-wins
//! semantics, `empty_input` verifies errors on zero-byte streams, `fixed_len_prefix`
//! tests the bound and debug check of a fixed-width length prefix, `invalid_strings`
//! tests UTF-8/UTF-16 rejection, `malformed_ber` tests BER error paths,
//! `oversized_length` tests declared-length overflow handling, `truncated`
//! tests short-read errors, and `zero_length_fields` tests `len = 0` edge cases
//!
//! Author: aav
// --------------------------------------------------
// mods
// --------------------------------------------------
mod boundary_values;
mod duplicate_keys;
mod empty_input;
mod fixed_len_prefix;
mod invalid_strings;
mod malformed_ber;
mod oversized_length;
mod truncated;
mod zero_length_fields;
