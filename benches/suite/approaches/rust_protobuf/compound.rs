//! rust-protobuf x compound record: the embedded coordinate goes through a
//! [`protobuf::MessageField`], the sensor run through a `Vec`; reverse on decode, requiring
//! the coordinate via `MessageField::into_option`.
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::{RustProtobuf, generated};
use crate::suite::Codec;
use crate::suite::records::{Compound, GpsCoord, Reading};

// --------------------------------------------------
// external
// --------------------------------------------------
use protobuf::Message as _;

/// [`RustProtobuf`] implementation of [`Codec`] for [`Compound`]
impl Codec<Compound> for RustProtobuf {
    fn encode(rec: &Compound) -> Vec<u8> {
        // --------------------------------------------------
        // build the proto message: MessageField coord + repeated sensors + widened scalars
        // --------------------------------------------------
        generated::Compound {
            id: rec.id,
            coord: protobuf::MessageField::some(generated::GpsCoord {
                lat: rec.coord.lat,
                lon: rec.coord.lon,
                ..Default::default()
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
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        }
        .write_to_bytes()
        .expect("write to Vec is infallible")
    }

    fn decode(body: &[u8]) -> Option<Compound> {
        // --------------------------------------------------
        // parse the proto message and require the embedded coordinate
        // --------------------------------------------------
        let m = generated::Compound::parse_from_bytes(body).ok()?;
        let coord = m.coord.into_option()?;
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
