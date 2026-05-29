//! tlv_parser x native record: build a BER-TLV tree (nested coordinate sub-tree `0x22` plus a
//! leaf per field, each native value reduced to raw bytes) to encode; walk by tag path and
//! hand-convert every leaf back to its native type to decode.
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::TlvParser;
use crate::suite::records::{GpsCoord, NativeNested};
use crate::suite::Codec;

// --------------------------------------------------
// external
// --------------------------------------------------
use chrono::{DateTime, Datelike, NaiveDate, NaiveTime, Timelike};
use core::num::NonZeroU32;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::time::Duration;
use ::tlv_parser::tlv::{Tlv, Value};

/// [`TlvParser`] implementation of [`Codec`] for [`NativeNested`]
impl Codec<NativeNested> for TlvParser {
    fn encode(rec: &NativeNested) -> Vec<u8> {
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
        // build top-level child list, reducing each native value to raw bytes by hand
        // --------------------------------------------------
        let ts = rec.timestamp.timestamp_nanos_opt().expect("timestamp in i64-ns range");
        let children = vec![
            Tlv::new(0x01, Value::Val(rec.id.to_be_bytes().to_vec())).unwrap(),
            coord,
            Tlv::new(0x03, Value::Val(ts.to_be_bytes().to_vec())).unwrap(),
            Tlv::new(0x04, Value::Val((rec.elapsed.as_nanos() as u64).to_be_bytes().to_vec())).unwrap(),
            Tlv::new(0x05, Value::Val(u32::from(rec.addr).to_be_bytes().to_vec())).unwrap(),
            Tlv::new(0x06, Value::Val(u128::from(rec.addr6).to_be_bytes().to_vec())).unwrap(),
            Tlv::new(0x07, Value::Val(rec.date.num_days_from_ce().to_be_bytes().to_vec())).unwrap(),
            Tlv::new(0x08, Value::Val(rec.time.num_seconds_from_midnight().to_be_bytes().to_vec())).unwrap(),
            Tlv::new(0x09, Value::Val(u32::from(rec.symbol).to_be_bytes().to_vec())).unwrap(),
            Tlv::new(0x10, Value::Val(rec.seq.get().to_be_bytes().to_vec())).unwrap(),
            Tlv::new(0x11, Value::Val(vec![u8::from(rec.flag)])).unwrap(),
            Tlv::new(0x12, Value::Val(rec.label.as_bytes().to_vec())).unwrap(),
        ];
        // --------------------------------------------------
        // wrap in constructed container tag and serialise
        // --------------------------------------------------
        Tlv::new(0x21, Value::TlvList(children)).unwrap().to_vec()
    }

    fn decode(body: &[u8]) -> Option<NativeNested> {
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
        // extract each leaf by tag path and hand-convert back to its native type
        // --------------------------------------------------
        Some(NativeNested {
            id: u32::from_be_bytes(get("21 / 01")?.as_slice().try_into().ok()?),
            coord: GpsCoord {
                lat: f64::from_be_bytes(get("21 / 22 / 01")?.as_slice().try_into().ok()?),
                lon: f64::from_be_bytes(get("21 / 22 / 02")?.as_slice().try_into().ok()?),
            },
            timestamp: DateTime::from_timestamp_nanos(i64::from_be_bytes(
                get("21 / 03")?.as_slice().try_into().ok()?,
            )),
            elapsed: Duration::from_nanos(u64::from_be_bytes(get("21 / 04")?.as_slice().try_into().ok()?)),
            addr: Ipv4Addr::from(u32::from_be_bytes(get("21 / 05")?.as_slice().try_into().ok()?)),
            addr6: Ipv6Addr::from(u128::from_be_bytes(get("21 / 06")?.as_slice().try_into().ok()?)),
            date: NaiveDate::from_num_days_from_ce_opt(i32::from_be_bytes(
                get("21 / 07")?.as_slice().try_into().ok()?,
            ))?,
            time: NaiveTime::from_num_seconds_from_midnight_opt(
                u32::from_be_bytes(get("21 / 08")?.as_slice().try_into().ok()?),
                0,
            )?,
            symbol: char::try_from(u32::from_be_bytes(get("21 / 09")?.as_slice().try_into().ok()?)).ok()?,
            seq: NonZeroU32::new(u32::from_be_bytes(get("21 / 10")?.as_slice().try_into().ok()?))?,
            flag: *get("21 / 11")?.first()? != 0,
            label: String::from_utf8(get("21 / 12")?.clone()).ok()?,
        })
    }
}
