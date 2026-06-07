//! tinyklv x compound record: the same native codecs as the simple record - the nested
//! coordinate and packed sensor run are composed by the derive, not by hand here.
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::Tinyklv;
use crate::suite::{Codec, records::Compound};
use ::tinyklv::prelude::*;

/// [`Tinyklv`] implementation of [`Codec`] for [`Compound`]
impl Codec<Compound> for Tinyklv {
    fn encode(rec: &Compound) -> Vec<u8> {
        let mut out = Vec::new();
        rec.encode_value(&mut out);
        out
    }

    fn decode(body: &[u8]) -> Option<Compound> {
        let mut input = body;
        Compound::decode_value(&mut input).ok()
    }

    fn encode_framed(rec: &Compound) -> Vec<u8> {
        let mut out = Vec::new();
        rec.encode_frame(&mut out);
        out
    }

    fn decode_framed(frame: &[u8]) -> Option<Compound> {
        let mut input = frame;
        Compound::decode_frame(&mut input).ok()
    }

    fn decode_streamed(buf: &[u8]) -> Option<Vec<Compound>> {
        let mut dec = Compound::decoder();
        dec.feed(buf);
        Some(dec.iter().collect())
    }
}
