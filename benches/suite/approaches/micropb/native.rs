//! micropb x native record: whole-struct conversion against the raw variant. micropb has no
//! std/chrono conversions, so the temporal fields are raw integers (`native_raw.proto`, under the
//! `bench_native_` package module) and every native type is reconstructed by hand.
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::generated::bench_native_;
use super::Micropb;
use crate::suite::records::{GpsCoord, NativeNested};
use crate::suite::Codec;

// --------------------------------------------------
// external
// --------------------------------------------------
use chrono::{DateTime, Datelike, NaiveDate, NaiveTime, Timelike};
use core::num::NonZeroU32;
use micropb::{MessageDecode as _, MessageEncode as _, PbEncoder};
use std::net::{Ipv4Addr, Ipv6Addr};
use std::time::Duration;

/// Raw (generated) → parsed (native): fallible, so the illegal-state checks live here.
impl TryFrom<bench_native_::NativeNested> for NativeNested {
    // `()` is deliberate: the sole consumer, `Codec::decode`, discards the reason via `.ok()`
    type Error = ();

    fn try_from(m: bench_native_::NativeNested) -> Result<Self, Self::Error> {
        // --------------------------------------------------
        // read the presence-tracked coordinate, then the rest
        // --------------------------------------------------
        let coord = m.coord().ok_or(())?;
        let coord = GpsCoord { lat: coord.lat, lon: coord.lon };
        let addr6 = Ipv6Addr::from(<[u8; 16]>::try_from(m.addr6.as_slice()).map_err(|_| ())?);
        Ok(NativeNested {
            id: m.id,
            coord,
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
impl From<&NativeNested> for bench_native_::NativeNested {
    // `set_coord` flips the presence hazzer, so build via default-then-populate
    #[allow(clippy::field_reassign_with_default)]
    fn from(rec: &NativeNested) -> Self {
        let mut m = bench_native_::NativeNested::default();
        m.id = rec.id;
        m.set_coord(bench_native_::GpsCoord {
            lat: rec.coord.lat,
            lon: rec.coord.lon,
        });
        m.timestamp_nanos = rec.timestamp.timestamp_nanos_opt().expect("timestamp in i64-ns range");
        m.elapsed_nanos = rec.elapsed.as_nanos() as u64;
        m.addr = u32::from(rec.addr);
        m.addr6 = rec.addr6.octets().to_vec();
        m.date_days = rec.date.num_days_from_ce();
        m.time_secs = rec.time.num_seconds_from_midnight();
        m.symbol = u32::from(rec.symbol);
        m.seq = rec.seq.get();
        m.flag = rec.flag;
        m.label.clone_from(&rec.label);
        m
    }
}

/// [`Micropb`] implementation of [`Codec`] for [`NativeNested`]
impl Codec<NativeNested> for Micropb {
    fn encode(rec: &NativeNested) -> Vec<u8> {
        let msg = bench_native_::NativeNested::from(rec);
        let mut encoder = PbEncoder::new(Vec::new());
        msg.encode(&mut encoder)
            .expect("write to Vec is infallible");
        encoder.into_writer()
    }

    fn decode(body: &[u8]) -> Option<NativeNested> {
        let mut m = bench_native_::NativeNested::default();
        m.decode_from_bytes(body).ok()?;
        m.try_into().ok()
    }
}
