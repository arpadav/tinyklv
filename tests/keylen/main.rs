//! Fused `#[klv(keylen(..))]` integration-test harness
//!
//! Declares the fused-codec test sub-modules so `cargo test` can discover them.
//! `pb` holds hand-written protobuf-shaped wire codecs, `example` the canonical
//! message built on them and the small `Reading` most tests share, `drive` the
//! generic decode drivers; the remaining sub-modules each cover one slice of the
//! fused codec
//!
//! Author: aav
// --------------------------------------------------
// mods
// --------------------------------------------------
mod decode_basic;
mod drive;
mod encode_roundtrip;
mod example;
mod frame_dispatch;
mod framing_rules;
mod macro_hygiene;
mod pb;
