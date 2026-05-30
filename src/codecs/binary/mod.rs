//! Binary codec module for KLV data
//!
//! Re-exports the [`dec`] and [`enc`] sub-modules: big-endian, little-endian, and
//! native-endian encoders/decoders for the Rust primitive integer and float types.
//!
//! For a field whose byte width is a Rust compile-time constant, use the free functions
//! in [`dec`] / [`enc`] directly (e.g. `tinyklv::codecs::binary::dec::be_u32`); for a
//! width that is fixed but narrower/wider than the native type, use the `*_lengthed`
//! variants.
//!
//! Author: aav
// --------------------------------------------------
// mods
// --------------------------------------------------
pub mod dec;
pub mod enc;
