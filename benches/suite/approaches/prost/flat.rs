//! prost x flat record: build the generated `Telemetry` message from the KLV record, then
//! `encode_to_vec`; on decode, parse and narrow the widened proto scalars back to KLV widths.
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::{generated, Prost};
use crate::suite::{records::Telemetry, Codec};

// --------------------------------------------------
// external
// --------------------------------------------------
use prost::Message as _;

/// [`Prost`] implementation of [`Codec`] for [`Telemetry`]
impl Codec<Telemetry> for Prost {
    fn encode(rec: &Telemetry) -> Vec<u8> {
        // --------------------------------------------------
        // widen KLV scalars into the proto message and serialize
        // --------------------------------------------------
        generated::Telemetry {
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

    fn decode(body: &[u8]) -> Option<Telemetry> {
        // --------------------------------------------------
        // parse the proto message
        // --------------------------------------------------
        let m = generated::Telemetry::decode(body).ok()?;
        // --------------------------------------------------
        // narrow widened scalars back to KLV widths, failing closed on overflow
        // --------------------------------------------------
        Some(Telemetry {
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
