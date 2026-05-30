//! tinyklv x rich record: native value and frame codecs, no hand-written glue. The
//! `Rich` fields (`DateTime<Utc>`, `Ipv4Addr`, `NonZeroU32`, ...) are decoded straight
//! into their final types via the `bench`-gated trait impls - compare this file's size to the
//! protobuf approaches' whole-struct conversions.
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::Tinyklv;
use crate::suite::{Codec, records::Rich};
use ::tinyklv::prelude::*;

/// [`Tinyklv`] implementation of [`Codec`] for [`Rich`]
impl Codec<Rich> for Tinyklv {
    fn encode(rec: &Rich) -> Vec<u8> {
        let mut out = Vec::new();
        rec.encode_value(&mut out);
        out
    }

    fn decode(body: &[u8]) -> Option<Rich> {
        let mut input = body;
        Rich::decode_value(&mut input).ok()
    }

    fn encode_framed(rec: &Rich) -> Vec<u8> {
        let mut out = Vec::new();
        rec.encode_frame(&mut out);
        out
    }

    fn decode_framed(frame: &[u8]) -> Option<Rich> {
        let mut input = frame;
        Rich::decode_frame(&mut input).ok()
    }

    fn decode_streamed(buf: &[u8]) -> Option<Vec<Rich>> {
        let mut dec = Rich::decoder();
        dec.feed(buf);
        Some(dec.iter().collect())
    }
}
