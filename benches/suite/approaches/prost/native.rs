//! prost x native record: a whole-struct conversion between the generated message and the
//! native record - protobuf auto-generates the struct, so its fields can never be native types.
//! Utilizes prost-types' real native paths: WKT `Duration` ↔ `std::time::Duration` (exact) and
//! WKT `Timestamp` ↔ `SystemTime` (then chrono's `From<SystemTime>` finishes `DateTime<Utc>`).
//! The remaining native types have no protobuf support, so they convert manually.
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::{generated, Prost};
use crate::suite::records::{GpsCoord, NativeNested};
use crate::suite::Codec;

// --------------------------------------------------
// external
// --------------------------------------------------
use chrono::{DateTime, Datelike, NaiveDate, NaiveTime, Timelike, Utc};
use core::num::NonZeroU32;
use prost::Message as _;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::time::{Duration, SystemTime};

/// Raw (generated) → parsed (native): fallible, so the illegal-state checks live here.
impl TryFrom<generated::NativeNested> for NativeNested {
    // `()` is deliberate: the sole consumer, `Codec::decode`, discards the reason via `.ok()`
    type Error = ();

    fn try_from(m: generated::NativeNested) -> Result<Self, Self::Error> {
        // --------------------------------------------------
        // crate-native temporal conversions: Timestamp -> SystemTime -> DateTime, WKT Duration
        // --------------------------------------------------
        let timestamp = DateTime::<Utc>::from(SystemTime::try_from(m.timestamp.ok_or(())?).map_err(|_| ())?);
        let elapsed = Duration::try_from(m.elapsed.ok_or(())?).map_err(|_| ())?;
        let coord = m.coord.ok_or(())?;
        let addr6 = Ipv6Addr::from(<[u8; 16]>::try_from(m.addr6.as_slice()).map_err(|_| ())?);
        Ok(NativeNested {
            id: m.id,
            coord: GpsCoord { lat: coord.lat, lon: coord.lon },
            timestamp,
            elapsed,
            addr: Ipv4Addr::from(m.addr),
            addr6,
            date: NaiveDate::from_num_days_from_ce_opt(m.date_days).ok_or(())?,
            time: NaiveTime::from_num_seconds_from_midnight_opt(m.time_secs, 0).ok_or(())?,
            symbol: char::try_from(m.symbol).map_err(|_| ())?,
            seq: NonZeroU32::new(m.seq).ok_or(())?,
            flag: m.flag,
            label: m.label,
        })
    }
}

/// Parsed (native) → raw (generated): infallible for the bench's bounded values.
impl From<&NativeNested> for generated::NativeNested {
    fn from(rec: &NativeNested) -> Self {
        generated::NativeNested {
            id: rec.id,
            coord: Some(generated::bench_native::GpsCoord {
                lat: rec.coord.lat,
                lon: rec.coord.lon,
            }),
            timestamp: Some(prost_types::Timestamp::from(SystemTime::from(rec.timestamp))),
            elapsed: Some(prost_types::Duration::try_from(rec.elapsed).expect("duration in WKT range")),
            addr: u32::from(rec.addr),
            addr6: rec.addr6.octets().to_vec(),
            date_days: rec.date.num_days_from_ce(),
            time_secs: rec.time.num_seconds_from_midnight(),
            symbol: u32::from(rec.symbol),
            seq: rec.seq.get(),
            flag: rec.flag,
            label: rec.label.clone(),
        }
    }
}

/// [`Prost`] implementation of [`Codec`] for [`NativeNested`]
impl Codec<NativeNested> for Prost {
    fn encode(rec: &NativeNested) -> Vec<u8> {
        generated::NativeNested::from(rec).encode_to_vec()
    }

    fn decode(body: &[u8]) -> Option<NativeNested> {
        generated::NativeNested::decode(body).ok()?.try_into().ok()
    }
}
