//! The tlv_parser approach: it parses bytes into a generic BER-TLV tree but gives you no
//! typed struct, so each record needs a hand-built tree to encode and a hand-converted
//! tree walk to decode (see [`flat`] / [`nested`]). It has no native framing, so it
//! inherits the shared seek-based `encode_framed`/`decode_framed`.
//!
//! Author: aav
// --------------------------------------------------
// mods
// --------------------------------------------------
mod compound;
mod rich;
mod simple;

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::suite::Approach;

/// The tlv_parser approach.
pub(crate) struct TlvParser;

/// [`TlvParser`] implementation of [`Approach`]
impl Approach for TlvParser {
    const NAME: &'static str = "tlv_parser";
}
