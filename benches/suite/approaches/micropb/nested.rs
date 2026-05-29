//! micropb x nested record: the embedded coordinate is set through the generated `set_coord`
//! builder (to flip its `_has` presence bit), the sensor run assigned as a `Vec`; reverse on
//! decode, requiring the coordinate via the `coord` accessor.
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::generated::bench_;
use super::Micropb;
use crate::suite::records::{GpsCoord, Platform, Reading};
use crate::suite::Codec;

// --------------------------------------------------
// external
// --------------------------------------------------
use micropb::{MessageDecode as _, MessageEncode as _, PbEncoder};

/// [`Micropb`] implementation of [`Codec`] for [`Platform`]
impl Codec<Platform> for Micropb {
    // a struct literal can't build Platform: `coord` presence lives in the `_has` hazzer and
    // is only flipped by the generated `set_coord`, so default-then-populate is required here
    #[allow(clippy::field_reassign_with_default)]
    fn encode(rec: &Platform) -> Vec<u8> {
        // --------------------------------------------------
        // build the proto message: set_coord flips the presence hazzer for the sub-packet
        // --------------------------------------------------
        let mut msg = bench_::Platform::default();
        msg.id = rec.id;
        msg.set_coord(bench_::GpsCoord {
            lat: rec.coord.lat,
            lon: rec.coord.lon,
        });
        msg.vx = rec.vx.into();
        msg.vy = rec.vy.into();
        msg.vz = rec.vz.into();
        msg.altitude = rec.altitude;
        msg.heading = rec.heading;
        msg.mode = rec.mode.into();
        msg.sensors = rec
            .sensors
            .iter()
            .map(|r| bench_::Reading {
                kind: r.kind.into(),
                value: r.value,
            })
            .collect();
        // --------------------------------------------------
        // encode through a PbEncoder over the output buffer
        // --------------------------------------------------
        let mut encoder = PbEncoder::new(Vec::new());
        msg.encode(&mut encoder)
            .expect("write to Vec is infallible");
        encoder.into_writer()
    }

    fn decode(body: &[u8]) -> Option<Platform> {
        // --------------------------------------------------
        // decode-merge the proto message and require the embedded coordinate
        // --------------------------------------------------
        let mut m = bench_::Platform::default();
        m.decode_from_bytes(body).ok()?;
        let c = m.coord()?;
        let coord = GpsCoord {
            lat: c.lat,
            lon: c.lon,
        };
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
        // reconstruct the nested record, narrowing the widened scalars
        // --------------------------------------------------
        Some(Platform {
            id: m.id,
            coord,
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
