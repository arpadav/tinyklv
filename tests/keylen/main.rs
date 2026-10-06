//! Fused `#[klv(keylen(..))]` integration-test harness
//!
//! Declares the fused-codec test sub-modules so `cargo test` can discover them. `support` holds
//! the codecs, messages and drivers they share; the remaining sub-modules each cover one slice of
//! the fused codec
//!
//! Author: aav
// --------------------------------------------------
// mods
// --------------------------------------------------
mod const_locals;
mod decode_basic;
mod encode_roundtrip;
mod frame_dispatch;
mod frame_progress;
mod framing_rules;
mod inline_error;
mod macro_hygiene;
mod support;
