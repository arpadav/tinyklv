//! quick-protobuf x flat record: build the generated `Telemetry`, write it through a
//! `Writer` into a `Vec`, then read it back with `from_reader` and narrow on decode.
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::{generated, QuickProtobuf};
use crate::suite::{records::Telemetry, Codec};

// --------------------------------------------------
// external
// --------------------------------------------------
use quick_protobuf::{BytesReader, MessageRead as _, MessageWrite as _, Writer};

/// [`QuickProtobuf`] implementation of [`Codec`] for [`Telemetry`]
impl Codec<Telemetry> for QuickProtobuf {
    fn encode(rec: &Telemetry) -> Vec<u8> {
        // --------------------------------------------------
        // widen KLV scalars into the proto message
        // --------------------------------------------------
        let msg = generated::Telemetry {
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
        // serialize through a Writer over the output buffer
        // --------------------------------------------------
        let mut out = Vec::new();
        {
            let mut writer = Writer::new(&mut out);
            msg.write_message(&mut writer)
                .expect("write to Vec is infallible");
        }
        out
    }

    fn decode(body: &[u8]) -> Option<Telemetry> {
        // --------------------------------------------------
        // parse the proto message from the input slice
        // --------------------------------------------------
        let mut reader = BytesReader::from_bytes(body);
        let m = generated::Telemetry::from_reader(&mut reader, body).ok()?;
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
