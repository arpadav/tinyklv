//! quick-protobuf x rich record: whole-struct conversion. quick-protobuf has no std/chrono
//! conversions, so the temporal fields are raw integers (the raw-scalar `Rich` schema) and every native
//! type is reconstructed by hand.
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::{QuickProtobuf, generated};
use crate::suite::Codec;
use crate::suite::records::{GpsCoord, Rich};

// --------------------------------------------------
// external
// --------------------------------------------------
use chrono::{DateTime, Datelike, NaiveDate, NaiveTime, Timelike};
use core::num::NonZeroU32;
use quick_protobuf::{BytesReader, MessageRead as _, MessageWrite as _, Writer};
use std::net::{Ipv4Addr, Ipv6Addr};
use std::time::Duration;

/// Raw (generated) → parsed (native): fallible, so the illegal-state checks live here.
impl TryFrom<generated::Rich> for Rich {
    // `()` is deliberate: the sole consumer, `Codec::decode`, discards the reason via `.ok()`
    type Error = ();

    fn try_from(m: generated::Rich) -> Result<Self, Self::Error> {
        let coord = m.coord.ok_or(())?;
        let addr6 = Ipv6Addr::from(<[u8; 16]>::try_from(m.addr6.as_slice()).map_err(|_| ())?);
        Ok(Rich {
            id: m.id,
            coord: GpsCoord {
                lat: coord.lat,
                lon: coord.lon,
            },
            timestamp: DateTime::from_timestamp_nanos(m.timestamp_nanos),
            elapsed: Duration::from_nanos(m.elapsed_nanos),
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
impl From<&Rich> for generated::Rich {
    fn from(rec: &Rich) -> Self {
        generated::Rich {
            id: rec.id,
            coord: Some(generated::native::GpsCoord {
                lat: rec.coord.lat,
                lon: rec.coord.lon,
            }),
            timestamp_nanos: rec
                .timestamp
                .timestamp_nanos_opt()
                .expect("timestamp in i64-ns range"),
            elapsed_nanos: rec.elapsed.as_nanos() as u64,
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

/// [`QuickProtobuf`] implementation of [`Codec`] for [`Rich`]
impl Codec<Rich> for QuickProtobuf {
    fn encode(rec: &Rich) -> Vec<u8> {
        let msg = generated::Rich::from(rec);
        let mut out = Vec::new();
        {
            let mut writer = Writer::new(&mut out);
            msg.write_message(&mut writer)
                .expect("write to Vec is infallible");
        }
        out
    }

    fn decode(body: &[u8]) -> Option<Rich> {
        let mut reader = BytesReader::from_bytes(body);
        generated::Rich::from_reader(&mut reader, body)
            .ok()?
            .try_into()
            .ok()
    }
}
