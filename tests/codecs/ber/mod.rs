//! BER codec tests
//!
//! `length` and `oid` cover BER variable-length encoding, `width` the BER wires built from each
//! integer width, and `standard` the examples the BER standards give
//!
//! `helpers` holds the shared `DecoderExt` value-and-cursor assertions
//!
//! Author: aav
// --------------------------------------------------
// mods
// --------------------------------------------------
mod helpers;
mod length;
mod oid;
mod standard;
mod width;
