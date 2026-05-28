//! The four benchmarked approaches, one directory each.
//!
//! Every approach directory holds an identically-named `flat.rs` / `nested.rs` pair
//! (`impl Codec<Telemetry>` / `impl Codec<Platform>`) plus its `mod.rs` (the unit struct
//! and its [`Approach`](crate::suite::Approach) identity). Open the four `flat.rs` files
//! side by side to compare how each approach handles the flat record - same method names,
//! same order.
//!
//! Author: aav
// --------------------------------------------------
// mods
// --------------------------------------------------
pub(crate) mod manual;
pub(crate) mod serde_klv;
pub(crate) mod tinyklv;
pub(crate) mod tlv_parser;

// --------------------------------------------------
// re-exports
// --------------------------------------------------
pub(crate) use manual::Manual;
pub(crate) use serde_klv::SerdeKlv;
pub(crate) use tinyklv::Tinyklv;
pub(crate) use tlv_parser::TlvParser;
