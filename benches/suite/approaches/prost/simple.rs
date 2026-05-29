//! prost x simple record: build the generated `Simple` message from the KLV record, then
//! `encode_to_vec`; on decode, parse and narrow the widened proto scalars back to KLV widths.
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::{Prost, generated};
use crate::suite::{Codec, records::Simple};

// --------------------------------------------------
// external
// --------------------------------------------------
use prost::Message as _;

/// [`Prost`] implementation of [`Codec`] for [`Simple`]
impl Codec<Simple> for Prost {
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
        }
        .encode_to_vec()
    }

    fn decode(body: &[u8]) -> Option<Simple> {
        // --------------------------------------------------
        // parse the proto message
        // --------------------------------------------------
        let m = generated::Simple::decode(body).ok()?;
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
