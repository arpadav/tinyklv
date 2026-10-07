//! BER codec tests
//!
//! `length` and `oid` cover BER variable-length encoding, and `width` the BER wires built from
//! each integer width
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
mod width;
