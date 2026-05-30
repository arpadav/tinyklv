//! String codec module for KLV data
//!
//! Re-exports the [`dec`] and [`enc`] sub-modules: variable-length string
//! decoders and encoders for UTF-8, UTF-16 (LE/BE), and ASCII text, plus
//! ASCII-encoded numeric values (base-10 integers, floats, and base-16 hex)
//!
//! Use the free functions in [`dec`] / [`enc`] directly (e.g
//! `tinyklv::codecs::string::dec::to_string_utf8`) in `#[klv(dec = ...)]` and
//! `#[klv(enc = ...)]` attributes. The ASCII-value decoders require the `ascii`
//! feature to be enabled
//!
//! Author: aav
pub mod dec;
pub mod enc;
