//! rust-protobuf x simple record: populate the generated `Simple` (with
//! `..Default::default()` for the hidden special fields), `write_to_bytes`, then
//! `parse_from_bytes` and narrow back on decode.
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::{RustProtobuf, generated};
use crate::suite::{Codec, records::Simple};

// --------------------------------------------------
// external
// --------------------------------------------------
use protobuf::Message as _;

/// [`RustProtobuf`] implementation of [`Codec`] for [`Simple`]
impl Codec<Simple> for RustProtobuf {
    fn encode(rec: &Simple) -> Vec<u8> {
        // --------------------------------------------------
        // widen KLV scalars into the proto message and serialize
        // --------------------------------------------------
        generated::Simple {
            a: rec.a.into(),
            b: rec.b.into(),
            c: rec.c,
            d: rec.d,
            e: rec.e.into(),
            f: rec.f,
            g: rec.g,
            h: rec.h,
            ..Default::default()
        }
        .write_to_bytes()
        .expect("write to Vec is infallible")
    }

    fn decode(body: &[u8]) -> Option<Simple> {
        // --------------------------------------------------
        // parse the proto message
        // --------------------------------------------------
        let m = generated::Simple::parse_from_bytes(body).ok()?;
        // --------------------------------------------------
        // narrow widened scalars back to KLV widths, failing closed on overflow
        // --------------------------------------------------
        Some(Simple {
            a: u8::try_from(m.a).ok()?,
            b: u16::try_from(m.b).ok()?,
            c: m.c,
            d: m.d,
            e: i16::try_from(m.e).ok()?,
            f: m.f,
            g: m.g,
            h: m.h,
        })
    }
}
