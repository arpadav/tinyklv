//! tlv_parser x nested record: build a nested tree (a constructed coordinate sub-tree
//! `0x22` plus packed sensors) to encode; walk by tag path and hand-convert every leaf -
//! including the nested coordinate path and the packed sensor run - to decode.
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::TlvParser;
use crate::suite::records::{GpsCoord, Platform, Reading};
use crate::suite::Codec;

// --------------------------------------------------
// external
// --------------------------------------------------
use ::tlv_parser::tlv::{Tlv, Value};

/// [`TlvParser`] implementation of [`Codec`] for [`Platform`]
impl Codec<Platform> for TlvParser {
    fn encode(rec: &Platform) -> Vec<u8> {
        // --------------------------------------------------
        // build nested coordinate sub-tree (tag 0x22)
        // --------------------------------------------------
        let coord = Tlv::new(
            0x22,
            Value::TlvList(vec![
                Tlv::new(0x01, Value::Val(rec.coord.lat.to_be_bytes().to_vec())).unwrap(),
                Tlv::new(0x02, Value::Val(rec.coord.lon.to_be_bytes().to_vec())).unwrap(),
            ]),
        )
        .unwrap();
        // --------------------------------------------------
        // build top-level child list including coord sub-tree
        // --------------------------------------------------
        let children = vec![
            Tlv::new(0x01, Value::Val(rec.id.to_be_bytes().to_vec())).unwrap(),
            coord,
            Tlv::new(0x03, Value::Val(rec.vx.to_be_bytes().to_vec())).unwrap(),
            Tlv::new(0x04, Value::Val(rec.vy.to_be_bytes().to_vec())).unwrap(),
            Tlv::new(0x05, Value::Val(rec.vz.to_be_bytes().to_vec())).unwrap(),
            Tlv::new(0x06, Value::Val(rec.altitude.to_be_bytes().to_vec())).unwrap(),
            Tlv::new(0x07, Value::Val(rec.heading.to_be_bytes().to_vec())).unwrap(),
            Tlv::new(0x08, Value::Val(vec![rec.mode])).unwrap(),
            Tlv::new(0x09, Value::Val(Reading::pack(&rec.sensors))).unwrap(),
        ];
        // --------------------------------------------------
        // wrap in constructed container tag and serialise
        // --------------------------------------------------
        Tlv::new(0x21, Value::TlvList(children)).unwrap().to_vec()
    }

    fn decode(body: &[u8]) -> Option<Platform> {
        // --------------------------------------------------
        // parse raw bytes into a TLV tree
        // --------------------------------------------------
        let tlv = Tlv::from_vec(body).ok()?;
        // --------------------------------------------------
        // build path-lookup helper closure
        // --------------------------------------------------
        let get = |path: &str| -> Option<&Vec<u8>> {
            match tlv.find_val(path) {
                Some(Value::Val(v)) => Some(v),
                _ => None,
            }
        };
        // --------------------------------------------------
        // unpack sensor run before constructing record
        // --------------------------------------------------
        let sensors = Reading::unpack(get("21 / 09")?)?;
        // --------------------------------------------------
        // extract each field by tag path and construct record
        // --------------------------------------------------
        Some(Platform {
            id: u32::from_be_bytes(get("21 / 01")?.as_slice().try_into().ok()?),
            coord: GpsCoord {
                lat: f64::from_be_bytes(get("21 / 22 / 01")?.as_slice().try_into().ok()?),
                lon: f64::from_be_bytes(get("21 / 22 / 02")?.as_slice().try_into().ok()?),
            },
            vx: i16::from_be_bytes(get("21 / 03")?.as_slice().try_into().ok()?),
            vy: i16::from_be_bytes(get("21 / 04")?.as_slice().try_into().ok()?),
            vz: i16::from_be_bytes(get("21 / 05")?.as_slice().try_into().ok()?),
            altitude: f64::from_be_bytes(get("21 / 06")?.as_slice().try_into().ok()?),
            heading: f32::from_be_bytes(get("21 / 07")?.as_slice().try_into().ok()?),
            mode: *get("21 / 08")?.first()?,
            sensors,
        })
    }
}
