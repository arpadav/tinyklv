//! tlv_parser x flat record: build a constructed tag `0x21` wrapping eight primitive
//! children to encode; walk the parsed tree and hand-convert each leaf to decode.
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::TlvParser;
use crate::suite::{records::Telemetry, Codec};

// --------------------------------------------------
// external
// --------------------------------------------------
use ::tlv_parser::tlv::{Tlv, Value};

/// [`TlvParser`] implementation of [`Codec`] for [`Telemetry`]
impl Codec<Telemetry> for TlvParser {
    fn encode(rec: &Telemetry) -> Vec<u8> {
        // --------------------------------------------------
        // build child TLV leaf nodes for each field
        // --------------------------------------------------
        let children = vec![
            Tlv::new(0x01, Value::Val(vec![rec.a])).unwrap(),
            Tlv::new(0x02, Value::Val(rec.b.to_be_bytes().to_vec())).unwrap(),
            Tlv::new(0x03, Value::Val(rec.c.to_be_bytes().to_vec())).unwrap(),
            Tlv::new(0x04, Value::Val(rec.d.to_be_bytes().to_vec())).unwrap(),
            Tlv::new(0x05, Value::Val(rec.e.to_be_bytes().to_vec())).unwrap(),
            Tlv::new(0x06, Value::Val(rec.f.to_be_bytes().to_vec())).unwrap(),
            Tlv::new(0x07, Value::Val(rec.g.to_be_bytes().to_vec())).unwrap(),
            Tlv::new(0x08, Value::Val(rec.h.to_be_bytes().to_vec())).unwrap(),
        ];
        // --------------------------------------------------
        // wrap in constructed container tag and serialise
        // --------------------------------------------------
        Tlv::new(0x21, Value::TlvList(children)).unwrap().to_vec()
    }

    fn decode(body: &[u8]) -> Option<Telemetry> {
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
        // extract each field by tag path and construct record
        // --------------------------------------------------
        Some(Telemetry {
            a: *get("21 / 01")?.first()?,
            b: u16::from_be_bytes(get("21 / 02")?.as_slice().try_into().ok()?),
            c: u32::from_be_bytes(get("21 / 03")?.as_slice().try_into().ok()?),
            d: u64::from_be_bytes(get("21 / 04")?.as_slice().try_into().ok()?),
            e: i16::from_be_bytes(get("21 / 05")?.as_slice().try_into().ok()?),
            f: i32::from_be_bytes(get("21 / 06")?.as_slice().try_into().ok()?),
            g: f32::from_be_bytes(get("21 / 07")?.as_slice().try_into().ok()?),
            h: f64::from_be_bytes(get("21 / 08")?.as_slice().try_into().ok()?),
        })
    }
}
