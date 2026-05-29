//! manual x native record: hand-pushed key/length/value triples, converting each native field
//! to its raw wire form inline; a hand-rolled tag loop converts each back on decode. The native
//! types buy nothing here - every field is reduced to bytes by hand.
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::{put, Manual};
use crate::suite::records::{GpsCoord, NativeNested};
use crate::suite::Codec;

// --------------------------------------------------
// external
// --------------------------------------------------
use chrono::{DateTime, Datelike, NaiveDate, NaiveTime, Timelike};
use core::num::NonZeroU32;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::time::Duration;

mod key {
    /// Key for field `id` (`u32`)
    pub const ID: u8 = 0x01;
    /// Key for the nested `GpsCoord` sub-packet
    pub const COORD: u8 = 0x02;
    /// Key for field `timestamp` (`DateTime<Utc>` as i64 ns)
    pub const TIMESTAMP: u8 = 0x03;
    /// Key for field `elapsed` (`Duration` as u64 ns)
    pub const ELAPSED: u8 = 0x04;
    /// Key for field `addr` (`Ipv4Addr` as u32)
    pub const ADDR: u8 = 0x05;
    /// Key for field `addr6` (`Ipv6Addr` as u128)
    pub const ADDR6: u8 = 0x06;
    /// Key for field `date` (`NaiveDate` as i32 days-from-CE)
    pub const DATE: u8 = 0x07;
    /// Key for field `time` (`NaiveTime` as u32 secs-from-midnight)
    pub const TIME: u8 = 0x08;
    /// Key for field `symbol` (`char` as u32)
    pub const SYMBOL: u8 = 0x09;
    /// Key for field `seq` (`NonZeroU32` as u32)
    pub const SEQ: u8 = 0x0A;
    /// Key for field `flag` (`bool` as u8)
    pub const FLAG: u8 = 0x0B;
    /// Key for field `label` (`String` as utf-8)
    pub const LABEL: u8 = 0x0C;
}

mod coord_key {
    /// Key for the latitude field inside the nested coordinate sub-packet (`f64`)
    pub const LAT: u8 = 0x01;
    /// Key for the longitude field inside the nested coordinate sub-packet (`f64`)
    pub const LON: u8 = 0x02;
}

/// [`Manual`] implementation of [`Codec`] for [`NativeNested`]
impl Codec<NativeNested> for Manual {
    fn encode(rec: &NativeNested) -> Vec<u8> {
        // --------------------------------------------------
        // encode nested coordinate sub-packet
        // --------------------------------------------------
        let mut coord = Vec::new();
        put(&mut coord, coord_key::LAT, &rec.coord.lat.to_be_bytes());
        put(&mut coord, coord_key::LON, &rec.coord.lon.to_be_bytes());
        // --------------------------------------------------
        // push each field, reducing the native type to its raw wire form by hand
        // --------------------------------------------------
        let mut out = Vec::new();
        put(&mut out, key::ID, &rec.id.to_be_bytes());
        put(&mut out, key::COORD, &coord);
        let ts = rec.timestamp.timestamp_nanos_opt().expect("timestamp in i64-ns range");
        put(&mut out, key::TIMESTAMP, &ts.to_be_bytes());
        put(&mut out, key::ELAPSED, &(rec.elapsed.as_nanos() as u64).to_be_bytes());
        put(&mut out, key::ADDR, &u32::from(rec.addr).to_be_bytes());
        put(&mut out, key::ADDR6, &u128::from(rec.addr6).to_be_bytes());
        put(&mut out, key::DATE, &rec.date.num_days_from_ce().to_be_bytes());
        put(&mut out, key::TIME, &rec.time.num_seconds_from_midnight().to_be_bytes());
        put(&mut out, key::SYMBOL, &u32::from(rec.symbol).to_be_bytes());
        put(&mut out, key::SEQ, &rec.seq.get().to_be_bytes());
        put(&mut out, key::FLAG, &[u8::from(rec.flag)]);
        put(&mut out, key::LABEL, rec.label.as_bytes());
        out
    }

    fn decode(body: &[u8]) -> Option<NativeNested> {
        // --------------------------------------------------
        // initialise field accumulators
        // --------------------------------------------------
        let mut id = None;
        let mut coord = None;
        let mut timestamp = None;
        let mut elapsed = None;
        let mut addr = None;
        let mut addr6 = None;
        let mut date = None;
        let mut time = None;
        let mut symbol = None;
        let mut seq = None;
        let mut flag = None;
        let mut label = None;
        // --------------------------------------------------
        // walk key/length/value triples, hand-converting each back to its native type
        // --------------------------------------------------
        let mut j = 0;
        while j + 2 <= body.len() {
            let tag = body[j];
            let len = usize::from(body[j + 1]);
            j += 2;
            let val = body.get(j..j + len)?;
            j += len;
            match tag {
                key::ID => id = Some(u32::from_be_bytes(val.try_into().ok()?)),
                key::COORD => coord = Some(decode_coord(val)?),
                key::TIMESTAMP => {
                    timestamp = Some(DateTime::from_timestamp_nanos(i64::from_be_bytes(val.try_into().ok()?)));
                }
                key::ELAPSED => {
                    elapsed = Some(Duration::from_nanos(u64::from_be_bytes(val.try_into().ok()?)));
                }
                key::ADDR => addr = Some(Ipv4Addr::from(u32::from_be_bytes(val.try_into().ok()?))),
                key::ADDR6 => addr6 = Some(Ipv6Addr::from(u128::from_be_bytes(val.try_into().ok()?))),
                key::DATE => {
                    date = Some(NaiveDate::from_num_days_from_ce_opt(i32::from_be_bytes(val.try_into().ok()?))?);
                }
                key::TIME => {
                    time = Some(NaiveTime::from_num_seconds_from_midnight_opt(u32::from_be_bytes(val.try_into().ok()?), 0)?);
                }
                key::SYMBOL => symbol = Some(char::try_from(u32::from_be_bytes(val.try_into().ok()?)).ok()?),
                key::SEQ => seq = Some(NonZeroU32::new(u32::from_be_bytes(val.try_into().ok()?))?),
                key::FLAG => flag = Some(*val.first()? != 0),
                key::LABEL => label = Some(String::from_utf8(val.to_vec()).ok()?),
                _ => {}
            }
        }
        // --------------------------------------------------
        // construct record, propagating None on any missing field
        // --------------------------------------------------
        Some(NativeNested {
            id: id?,
            coord: coord?,
            timestamp: timestamp?,
            elapsed: elapsed?,
            addr: addr?,
            addr6: addr6?,
            date: date?,
            time: time?,
            symbol: symbol?,
            seq: seq?,
            flag: flag?,
            label: label?,
        })
    }
}

/// Decodes the nested coordinate sub-packet from its raw value body
///
/// # Arguments
///
/// * `body` - the raw value bytes for the coordinate sub-packet field
///
/// # Returns
///
/// `Some(GpsCoord)` when both `lat` and `lon` were found, else `None`
fn decode_coord(body: &[u8]) -> Option<GpsCoord> {
    // --------------------------------------------------
    // initialise coordinate field accumulators
    // --------------------------------------------------
    let mut lat = None;
    let mut lon = None;
    // --------------------------------------------------
    // walk sub-packet triples and dispatch by tag
    // --------------------------------------------------
    let mut j = 0;
    while j + 2 <= body.len() {
        let tag = body[j];
        let len = usize::from(body[j + 1]);
        j += 2;
        let val = body.get(j..j + len)?;
        j += len;
        match tag {
            coord_key::LAT => lat = Some(f64::from_be_bytes(val.try_into().ok()?)),
            coord_key::LON => lon = Some(f64::from_be_bytes(val.try_into().ok()?)),
            _ => {}
        }
    }
    // --------------------------------------------------
    // construct coordinate, propagating None on any missing field
    // --------------------------------------------------
    Some(GpsCoord {
        lat: lat?,
        lon: lon?,
    })
}
