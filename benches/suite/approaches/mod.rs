//! The eight benchmarked approaches, one directory each.
//!
//! Every approach directory holds an identically-named `flat.rs` / `nested.rs` pair
//! (`impl Codec<Telemetry>` / `impl Codec<Platform>`) plus its `mod.rs` (the unit struct
//! and its [`Approach`](crate::suite::Approach) identity). Open the eight `flat.rs` files
//! side by side to compare how each approach handles the flat record - same method names,
//! same order. The four KLV approaches (tinyklv, serde_klv, tlv_parser, manual) sit next to
//! four protobuf crates (prost, quick_protobuf, rust_protobuf, micropb), which round-trip the
//! same records through generated protobuf messages.
//!
//! Author: aav
// --------------------------------------------------
// mods
// --------------------------------------------------
pub(crate) mod manual;
pub(crate) mod micropb;
pub(crate) mod prost;
pub(crate) mod quick_protobuf;
pub(crate) mod rust_protobuf;
pub(crate) mod serde_klv;
pub(crate) mod tinyklv;
pub(crate) mod tlv_parser;

// --------------------------------------------------
// re-exports
// --------------------------------------------------
pub(crate) use manual::Manual;
pub(crate) use micropb::Micropb;
pub(crate) use prost::Prost;
pub(crate) use quick_protobuf::QuickProtobuf;
pub(crate) use rust_protobuf::RustProtobuf;
pub(crate) use serde_klv::SerdeKlv;
pub(crate) use tinyklv::Tinyklv;
pub(crate) use tlv_parser::TlvParser;
