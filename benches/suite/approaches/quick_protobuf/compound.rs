//! quick-protobuf x compound record: the embedded coordinate is an `Option<GpsCoord>`, the
//! sensor run a `Vec`; reverse on decode, requiring the coordinate.
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::{QuickProtobuf, generated};
use crate::suite::Codec;
use crate::suite::records::{Compound, GpsCoord, Reading};

// --------------------------------------------------
// external
// --------------------------------------------------
use quick_protobuf::{BytesReader, MessageRead as _, MessageWrite as _, Writer};

/// [`QuickProtobuf`] implementation of [`Codec`] for [`Compound`]
impl Codec<Compound> for QuickProtobuf {
    fn encode(rec: &Compound) -> Vec<u8> {
        // --------------------------------------------------
        // build the proto message: embedded coord + repeated sensors + widened scalars
        // --------------------------------------------------
        let msg = generated::Compound {
            id: rec.id,
            coord: Some(generated::GpsCoord {
                lat: rec.coord.lat,
                lon: rec.coord.lon,
            }),
            vx: rec.vx.into(),
            vy: rec.vy.into(),
            vz: rec.vz.into(),
            altitude: rec.altitude,
            heading: rec.heading,
            mode: rec.mode.into(),
            sensors: rec
                .sensors
                .iter()
                .map(|r| generated::Reading {
                    kind: r.kind.into(),
                    value: r.value,
                })
                .collect(),
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

    fn decode(body: &[u8]) -> Option<Compound> {
        // --------------------------------------------------
        // parse the proto message and require the embedded coordinate
        // --------------------------------------------------
        let mut reader = BytesReader::from_bytes(body);
        let m = generated::Compound::from_reader(&mut reader, body).ok()?;
        let coord = m.coord?;
        // --------------------------------------------------
        // narrow each sensor's kind, failing closed on overflow
        // --------------------------------------------------
        let sensors = m
            .sensors
            .iter()
            .map(|r| {
                Some(Reading {
                    kind: u8::try_from(r.kind).ok()?,
                    value: r.value,
                })
            })
            .collect::<Option<Vec<_>>>()?;
        // --------------------------------------------------
        // reconstruct the compound record, narrowing the widened scalars
        // --------------------------------------------------
        Some(Compound {
            id: m.id,
            coord: GpsCoord {
                lat: coord.lat,
                lon: coord.lon,
            },
            vx: i16::try_from(m.vx).ok()?,
            vy: i16::try_from(m.vy).ok()?,
            vz: i16::try_from(m.vz).ok()?,
            altitude: m.altitude,
            heading: m.heading,
            mode: u8::try_from(m.mode).ok()?,
            sensors,
        })
    }
}
