//! Derive macro integration-test harness
//!
//! Declares every `#[derive(Klv)]` test sub-module so `cargo test` can
//! discover them.  Sub-modules are grouped by feature area: `field` a
//! field's type and width, `codec` the codec and fallback a field decodes
//! with, `roundtrip` a whole struct's encode and decode, `streaming` partial
//! decode / resume, `Decoder` and vec-decode semantics, `sentinel` sentinel
//! seek and frame extraction, and `loop_control` what ends or rejects the
//! decode loop.  The `types` module provides shared domain types used across
//! the advanced sub-modules, `beacon` the all-optional message the
//! vec-decode sub-modules share, and `attempt_budget` the cap on decode
//! attempts that turns a hang into a panic
//!
//! Author: aav
// --------------------------------------------------
// mods
// --------------------------------------------------
mod attempt_budget;
mod beacon;
mod codec {
    mod advanced_defaults;
    mod ber_keyed;
    mod custom_decoders;
    mod default_values;
    mod fallback_trait_impl;
    mod latebind;
    mod sigil_dispatch;
}
mod field {
    mod advanced_nesting;
    mod advanced_variable;
    #[cfg(feature = "ascii")]
    mod ascii_fields;
    mod basic_fixed;
    mod basic_variable;
    mod mixed_types;
    mod nested_types;
    mod optional_fields;
    mod partial_decode;
}
mod loop_control {
    mod advanced_break;
    mod break_condition;
    mod break_on_attr;
    mod deny_unknown_keys;
    mod shadowed_matches;
    mod streaming_unknown_key;
}
mod roundtrip {
    mod advanced_generics;
    mod advanced_keys;
    mod advanced_roundtrip;
    mod advanced_scaling;
    mod encode_roundtrip;
    mod field_ordering;
}
mod sentinel {
    mod advanced_repeated;
    mod advanced_sentinel;
    mod advanced_stream;
    mod enum_dispatch;
    mod multi_packet;
    mod repeated_decode;
    mod sentinel_seek;
}
mod streaming {
    mod decode_partial_error_kinds;
    mod decode_partial_resume;
    mod streaming_decoder;
    mod streaming_mixed_fields;
    mod streaming_vec;
    mod vec_decode_value;
}
mod types;
