//! prost x compound record: map the embedded `GpsCoord` and the `repeated Reading` run into
//! the generated `Compound` message, then `encode_to_vec`; reverse on decode, narrowing
//! widened scalars and rejecting an absent coordinate.
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::{Prost, generated};
use crate::suite::Codec;
use crate::suite::records::{Compound, GpsCoord, Reading};

// --------------------------------------------------
// external
// --------------------------------------------------
use prost::Message as _;

/// [`Prost`] implementation of [`Codec`] for [`Compound`]
impl Codec<Compound> for Prost {
    fn encode(rec: &Compound) -> Vec<u8> {
        // --------------------------------------------------
        // build the proto message: embedded coord + repeated sensors + widened scalars
        // --------------------------------------------------
        generated::Compound {
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
        }
        .encode_to_vec()
    }

    fn decode(body: &[u8]) -> Option<Compound> {
        // --------------------------------------------------
        // parse the proto message and require the embedded coordinate
        // --------------------------------------------------
        let m = generated::Compound::decode(body).ok()?;
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
