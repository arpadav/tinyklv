//! serde_klv x rich record: serde_klv can only (de)serialize primitive/string fields, so the
//! rich record is hand-mapped through a flat `NativeWire` of raw scalars (coordinate flattened
//! to `lat`/`lon`, `addr6` as a byte blob) - field-by-field conversion in the `Serialize` /
//! `Deserialize` impls, the cost serde_klv pays to reach native types.
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::SerdeKlv;
use crate::suite::Codec;
use crate::suite::records::{GpsCoord, Rich};

// --------------------------------------------------
// external
// --------------------------------------------------
use chrono::{DateTime, Datelike, NaiveDate, NaiveTime, Timelike};
use core::num::NonZeroU32;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::net::{Ipv4Addr, Ipv6Addr};
use std::time::Duration;

/// Flat raw-scalar wire form of [`Rich`] that serde_klv can derive.
///
/// serde_klv cannot nest or hold native types, so the `GpsCoord` is flattened to `lat`/`lon`,
/// the temporals become raw integers, and `addr6` becomes a `serde_bytes` blob. Both directions
/// hand-convert between `Rich` and this wire form.
#[derive(Serialize, Deserialize)]
#[serde(rename = "NATIVE0000000000")]
struct NativeWire {
    #[serde(rename = "1")]
    id: u32,
    #[serde(rename = "2")]
    lat: f64,
    #[serde(rename = "3")]
    lon: f64,
    #[serde(rename = "4")]
    timestamp_nanos: i64,
    #[serde(rename = "5")]
    elapsed_nanos: u64,
    #[serde(rename = "6")]
    addr: u32,
    #[serde(rename = "7", with = "serde_bytes")]
    addr6: Vec<u8>,
    #[serde(rename = "8")]
    date_days: i32,
    #[serde(rename = "9")]
    time_secs: u32,
    #[serde(rename = "10")]
    symbol: u32,
    #[serde(rename = "11")]
    seq: u32,
    #[serde(rename = "12")]
    flag: bool,
    #[serde(rename = "13")]
    label: String,
}

/// [`Rich`] implementation of [`Serialize`]
impl Serialize for Rich {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // --------------------------------------------------
        // reduce each native field to its raw wire scalar and delegate to serde_klv
        // --------------------------------------------------
        NativeWire {
            id: self.id,
            lat: self.coord.lat,
            lon: self.coord.lon,
            timestamp_nanos: self
                .timestamp
                .timestamp_nanos_opt()
                .expect("timestamp in i64-ns range"),
            elapsed_nanos: self.elapsed.as_nanos() as u64,
            addr: u32::from(self.addr),
            addr6: self.addr6.octets().to_vec(),
            date_days: self.date.num_days_from_ce(),
            time_secs: self.time.num_seconds_from_midnight(),
            symbol: u32::from(self.symbol),
            seq: self.seq.get(),
            flag: self.flag,
            label: self.label.clone(),
        }
        .serialize(serializer)
    }
}

/// [`Rich`] implementation of [`Deserialize`]
impl<'de> Deserialize<'de> for Rich {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // --------------------------------------------------
        // deserialize the flat wire form, then hand-rebuild each native type (fail closed)
        // --------------------------------------------------
        let w = NativeWire::deserialize(deserializer)?;
        let addr6: [u8; 16] = w
            .addr6
            .as_slice()
            .try_into()
            .map_err(|_| D::Error::custom("addr6 must be 16 bytes"))?;
        Ok(Rich {
            id: w.id,
            coord: GpsCoord {
                lat: w.lat,
                lon: w.lon,
            },
            timestamp: DateTime::from_timestamp_nanos(w.timestamp_nanos),
            elapsed: Duration::from_nanos(w.elapsed_nanos),
            addr: Ipv4Addr::from(w.addr),
            addr6: Ipv6Addr::from(addr6),
            date: NaiveDate::from_num_days_from_ce_opt(w.date_days)
                .ok_or_else(|| D::Error::custom("date out of range"))?,
            time: NaiveTime::from_num_seconds_from_midnight_opt(w.time_secs, 0)
                .ok_or_else(|| D::Error::custom("time out of range"))?,
            symbol: char::try_from(w.symbol).map_err(|_| D::Error::custom("invalid char"))?,
            seq: NonZeroU32::new(w.seq).ok_or_else(|| D::Error::custom("seq must be non-zero"))?,
            flag: w.flag,
            label: w.label,
        })
    }
}

/// [`SerdeKlv`] implementation of [`Codec`] for [`Rich`]
impl Codec<Rich> for SerdeKlv {
    fn encode(rec: &Rich) -> Vec<u8> {
        ::serde_klv::to_bytes(rec).unwrap()
    }

    fn decode(body: &[u8]) -> Option<Rich> {
        ::serde_klv::from_bytes::<Rich>(body).ok()
    }
}
