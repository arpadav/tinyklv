//! serde_klv x flat record: `#[derive(Serialize, Deserialize)]` on the record carries the
//! whole codec - same as tinyklv, no adapter needed for a flat record.
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::SerdeKlv;
use crate::suite::{records::Telemetry, Codec};

/// [`SerdeKlv`] implementation of [`Codec`] for [`Telemetry`]
impl Codec<Telemetry> for SerdeKlv {
    fn encode(rec: &Telemetry) -> Vec<u8> {
        ::serde_klv::to_bytes(rec).unwrap()
    }

    fn decode(body: &[u8]) -> Option<Telemetry> {
        ::serde_klv::from_bytes::<Telemetry>(body).ok()
    }
}
