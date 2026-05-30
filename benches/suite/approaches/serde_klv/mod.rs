//! The serde_klv approach: a derive suffices for the simple record, but nesting forces a
//! hand-written `Serialize`/`Deserialize` (see [`nested`]). serde_klv has no native
//! framing, so it inherits the shared seek-based `encode_framed`/`decode_framed`.
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

/// The serde_klv approach.
pub(crate) struct SerdeKlv;

/// [`SerdeKlv`] implementation of [`Approach`]
impl Approach for SerdeKlv {
    const NAME: &'static str = "serde_klv";
}
