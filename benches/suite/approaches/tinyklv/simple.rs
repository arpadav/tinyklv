//! tinyklv x simple record: native value and frame codecs, no hand-written glue.
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::Tinyklv;
use crate::suite::{Codec, records::Simple};
use ::tinyklv::prelude::*;

/// [`Tinyklv`] implementation of [`Codec`] for [`Simple`]
impl Codec<Simple> for Tinyklv {
    fn encode(rec: &Simple) -> Vec<u8> {
        let mut out = Vec::new();
        rec.encode_value(&mut out);
        out
    }

    fn decode(body: &[u8]) -> Option<Simple> {
        let mut input = body;
        Simple::decode_value(&mut input).ok()
    }

    fn encode_framed(rec: &Simple) -> Vec<u8> {
        let mut out = Vec::new();
        rec.encode_frame(&mut out);
        out
    }

    fn decode_framed(frame: &[u8]) -> Option<Simple> {
        let mut input = frame;
        Simple::decode_frame(&mut input).ok()
    }

    fn decode_streamed(buf: &[u8]) -> Option<Vec<Simple>> {
        let mut dec = Simple::decoder();
        dec.feed(buf);
        Some(dec.iter().collect())
    }
}
