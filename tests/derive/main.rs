//! Derive macro integration-test harness
//!
//! Declares every `#[derive(Klv)]` test sub-module so `cargo test` can
//! discover them.  Sub-modules are organised by feature area: basic fixed
//! and variable fields, advanced streaming / sentinel / scaling / nesting,
//! ASCII-encoded fields, sigil dispatch, partial decode / resume, optional
//! fields, and vec-decode semantics.  The `types` module provides shared
//! domain types used across the advanced sub-modules
//!
//! Author: aav
// --------------------------------------------------
// mods
// --------------------------------------------------
mod advanced_break;
mod advanced_defaults;
mod advanced_generics;
mod advanced_keys;
mod advanced_nesting;
mod advanced_repeated;
mod advanced_roundtrip;
mod advanced_scaling;
mod advanced_sentinel;
mod advanced_stream;
mod advanced_variable;
#[cfg(feature = "ascii")]
mod ascii_fields;
mod basic_fixed;
mod basic_variable;
mod ber_keyed;
mod break_condition;
mod break_on_attr;
mod custom_decoders;
mod decode_partial_error_kinds;
mod decode_partial_resume;
mod default_values;
mod deny_unknown_keys;
mod encode_roundtrip;
mod enum_dispatch;
mod fallback_trait_impl;
mod field_ordering;
mod latebind;
mod mixed_types;
mod multi_packet;
mod nested_types;
mod optional_fields;
mod partial_decode;
mod repeated_decode;
mod sentinel_seek;
mod sigil_dispatch;
mod streaming_decoder;
mod streaming_mixed_fields;
mod streaming_unknown_key;
mod streaming_vec;
mod types;
mod vec_decode_value;
