//! serde_klv x simple record: `#[derive(Serialize, Deserialize)]` on the record carries the
//! whole codec - same as tinyklv, no adapter needed for a simple record.
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::SerdeKlv;
use crate::suite::{Codec, records::Simple};

/// [`SerdeKlv`] implementation of [`Codec`] for [`Simple`]
impl Codec<Simple> for SerdeKlv {
    fn encode(rec: &Simple) -> Vec<u8> {
        ::serde_klv::to_bytes(rec).unwrap()
    }

    fn decode(body: &[u8]) -> Option<Simple> {
        ::serde_klv::from_bytes::<Simple>(body).ok()
    }
}
