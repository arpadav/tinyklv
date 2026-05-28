//! tinyklv x flat record: native value and frame codecs, no hand-written glue.
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::Tinyklv;
use crate::suite::{records::Telemetry, Codec};
use ::tinyklv::prelude::*;

/// [`Tinyklv`] implementation of [`Codec`] for [`Telemetry`]
impl Codec<Telemetry> for Tinyklv {
    fn encode(rec: &Telemetry) -> Vec<u8> {
        rec.encode_value()
    }

    fn decode(body: &[u8]) -> Option<Telemetry> {
        let mut input = body;
        Telemetry::decode_value(&mut input).ok()
    }

    fn encode_framed(rec: &Telemetry) -> Vec<u8> {
        rec.encode_frame()
    }

    fn decode_framed(noisy: &[u8]) -> Option<Telemetry> {
        let mut input = noisy;
        Telemetry::decode_frame(&mut input).ok()
    }
}
