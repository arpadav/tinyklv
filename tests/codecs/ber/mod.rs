//! BER codec tests
//!
//! `length` and `oid` cover BER variable-length encoding, and `width` the BER wires built from
//! each integer width
//!
//! `helpers` holds the shared `ContextErrorExt` queries on a BER error's `Expected` context and
//! the `DecoderExt` value-and-cursor assertions
//!
//! Author: aav
// --------------------------------------------------
// mods
// --------------------------------------------------
mod helpers;
mod length;
mod oid;
mod width;
