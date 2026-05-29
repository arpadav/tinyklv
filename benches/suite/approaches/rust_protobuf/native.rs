//! rust-protobuf x native record: whole-struct conversion. Utilizes rust-protobuf's WKT native
//! paths - `Duration` ↔ `std::time::Duration` (exact) and `Timestamp` ↔ `SystemTime` (then chrono
//! finishes `DateTime<Utc>`). No chrono integration exists, and the other native types have no
//! protobuf support, so they convert manually.
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::{generated, RustProtobuf};
use crate::suite::records::{GpsCoord, NativeNested};
use crate::suite::Codec;

// --------------------------------------------------
// external
// --------------------------------------------------
use chrono::{DateTime, Datelike, NaiveDate, NaiveTime, Timelike, Utc};
use core::num::NonZeroU32;
use protobuf::well_known_types::duration::Duration as PbDuration;
use protobuf::well_known_types::timestamp::Timestamp as PbTimestamp;
use protobuf::Message as _;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::time::{Duration, SystemTime};

/// Raw (generated) → parsed (native): fallible, so the illegal-state checks live here.
impl TryFrom<generated::NativeNested> for NativeNested {
    // `()` is deliberate: the sole consumer, `Codec::decode`, discards the reason via `.ok()`
    type Error = ();

    fn try_from(m: generated::NativeNested) -> Result<Self, Self::Error> {
        // --------------------------------------------------
        // crate-native temporal conversions via well-known types
        // --------------------------------------------------
        let system_time: SystemTime = m.timestamp.into_option().ok_or(())?.into();
        let elapsed: Duration = m.elapsed.into_option().ok_or(())?.into();
        let coord = m.coord.into_option().ok_or(())?;
        let addr6 = Ipv6Addr::from(<[u8; 16]>::try_from(m.addr6.as_slice()).map_err(|_| ())?);
        Ok(NativeNested {
            id: m.id,
            coord: GpsCoord { lat: coord.lat, lon: coord.lon },
            timestamp: DateTime::<Utc>::from(system_time),
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
            coord: protobuf::MessageField::some(generated::native::GpsCoord {
                lat: rec.coord.lat,
                lon: rec.coord.lon,
                ..Default::default()
            }),
            timestamp: protobuf::MessageField::some(PbTimestamp::from(SystemTime::from(rec.timestamp))),
            elapsed: protobuf::MessageField::some(PbDuration::from(rec.elapsed)),
            addr: u32::from(rec.addr),
            addr6: rec.addr6.octets().to_vec(),
            date_days: rec.date.num_days_from_ce(),
            time_secs: rec.time.num_seconds_from_midnight(),
            symbol: u32::from(rec.symbol),
            seq: rec.seq.get(),
            flag: rec.flag,
            label: rec.label.clone(),
            ..Default::default()
        }
    }
}

/// [`RustProtobuf`] implementation of [`Codec`] for [`NativeNested`]
impl Codec<NativeNested> for RustProtobuf {
    fn encode(rec: &NativeNested) -> Vec<u8> {
        generated::NativeNested::from(rec)
            .write_to_bytes()
            .expect("write to Vec is infallible")
    }

    fn decode(body: &[u8]) -> Option<NativeNested> {
        generated::NativeNested::parse_from_bytes(body)
            .ok()?
            .try_into()
            .ok()
    }
}
