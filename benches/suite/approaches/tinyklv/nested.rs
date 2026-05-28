//! tinyklv x nested record: the same native codecs as the flat record - the nested
//! coordinate and packed sensor run are composed by the derive, not by hand here.
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::Tinyklv;
use crate::suite::{records::Platform, Codec};
use ::tinyklv::prelude::*;

/// [`Tinyklv`] implementation of [`Codec`] for [`Platform`]
impl Codec<Platform> for Tinyklv {
    fn encode(rec: &Platform) -> Vec<u8> {
        rec.encode_value()
    }

    fn decode(body: &[u8]) -> Option<Platform> {
        let mut input = body;
        Platform::decode_value(&mut input).ok()
    }

    fn encode_framed(rec: &Platform) -> Vec<u8> {
        rec.encode_frame()
    }

    fn decode_framed(noisy: &[u8]) -> Option<Platform> {
        let mut input = noisy;
        Platform::decode_frame(&mut input).ok()
    }
}
