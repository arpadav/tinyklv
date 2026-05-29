//! micropb x simple record: build the generated `Simple`, encode it through a `PbEncoder`
//! over a `Vec`, then decode-merge into a default and narrow on the way out.
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::Micropb;
use super::generated::bench_;
use crate::suite::{Codec, records::Simple};

// --------------------------------------------------
// external
// --------------------------------------------------
use micropb::{MessageDecode as _, MessageEncode as _, PbEncoder};

/// [`Micropb`] implementation of [`Codec`] for [`Simple`]
impl Codec<Simple> for Micropb {
    fn encode(rec: &Simple) -> Vec<u8> {
        // --------------------------------------------------
        // widen KLV scalars into the proto message
        // --------------------------------------------------
        let msg = bench_::Simple {
            a: rec.a.into(),
            b: rec.b.into(),
            c: rec.c,
            d: rec.d,
            e: rec.e.into(),
            f: rec.f,
            g: rec.g,
            h: rec.h,
        };
        // --------------------------------------------------
        // encode through a PbEncoder over the output buffer
        // --------------------------------------------------
        let mut encoder = PbEncoder::new(Vec::new());
        msg.encode(&mut encoder)
            .expect("write to Vec is infallible");
        encoder.into_writer()
    }

    fn decode(body: &[u8]) -> Option<Simple> {
        // --------------------------------------------------
        // decode-merge the proto message into a default instance
        // --------------------------------------------------
        let mut m = bench_::Simple::default();
        m.decode_from_bytes(body).ok()?;
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
