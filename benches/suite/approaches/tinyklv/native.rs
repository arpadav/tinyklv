//! tinyklv x native record: native value and frame codecs, no hand-written glue. The
//! `NativeNested` fields (`DateTime<Utc>`, `Ipv4Addr`, `NonZeroU32`, ...) are decoded straight
//! into their final types via the `bench`-gated trait impls - compare this file's size to the
//! protobuf approaches' whole-struct conversions.
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::Tinyklv;
use crate::suite::{records::NativeNested, Codec};
use ::tinyklv::prelude::*;

/// [`Tinyklv`] implementation of [`Codec`] for [`NativeNested`]
impl Codec<NativeNested> for Tinyklv {
    fn encode(rec: &NativeNested) -> Vec<u8> {
        rec.encode_value()
    }

    fn decode(body: &[u8]) -> Option<NativeNested> {
        let mut input = body;
        NativeNested::decode_value(&mut input).ok()
    }

    fn encode_framed(rec: &NativeNested) -> Vec<u8> {
        rec.encode_frame()
    }

    fn decode_framed(noisy: &[u8]) -> Option<NativeNested> {
        let mut input = noisy;
        NativeNested::decode_frame(&mut input).ok()
    }
}
