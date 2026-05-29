//! Native-Rust-type codecs (feature `bench`).
//!
//! Hand-written [`DecodeValue`](crate::DecodeValue) / [`EncodeValue`](crate::EncodeValue) impls
//! for common `std` / `chrono` types, so a derived KLV struct can use them as **first-class
//! fields** - `timestamp: DateTime<Utc>`, `addr: Ipv4Addr`, ... - and decode straight into them
//! with no per-field `dec`/`enc` attribute and no separate "raw" struct. This is only sound
//! *inside* tinyklv: the traits are ours, so impl-ing them for foreign types is coherence-legal
//! here (a downstream crate could not).
//!
//! Gated behind the opt-in `bench` feature; the name is provisional (this could be exposed as a
//! stable `native` surface one day). It exists today to drive the `NativeNested` benchmark record.
//!
//! Design notes:
//! * **Endianness is big-endian by fiat.** A trait impl is one-wire-form-per-type, unlike the
//!   primitive codecs which parameterize endianness at the call site via
//!   `default(dec = be_*/le_*)` free functions. BE is chosen to match the rest of the suite.
//! * **[`NaiveTime`] is lossy for leap seconds**: [`Timelike::num_seconds_from_midnight`] drops the
//!   leap nanosecond. Round-trips are exact only for non-leap times.
//! * The [`bool`] / [`char`] impls are a permanent semantic commitment while the feature is on
//!   (`bool` = `u8 != 0`, `char` = big-endian `u32`); revisit with newtypes if `bench` ever
//!   stabilizes into a public API.
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::codecs::binary::{dec as decb, enc as encb};
use crate::{DecodeValue, EncodeValue};

// --------------------------------------------------
// external
// --------------------------------------------------
use chrono::{DateTime, Datelike, NaiveDate, NaiveTime, Timelike, Utc};
use core::num::NonZeroU32;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::time::Duration;
use winnow::error::{ContextError, ParserError};

/// [`DateTime<Utc>`] codec: a big-endian `i64` of nanoseconds since the Unix epoch.
impl DecodeValue<&[u8]> for DateTime<Utc> {
    fn decode_value(input: &mut &[u8]) -> crate::Result<Self> {
        Ok(DateTime::from_timestamp_nanos(decb::be_i64(input)?))
    }
}
impl EncodeValue<Vec<u8>> for DateTime<Utc> {
    fn encode_value(&self) -> Vec<u8> {
        // infallible for the bounded timestamps `RngSample` emits (well inside the i64-ns range)
        encb::be_i64(self.timestamp_nanos_opt().expect("timestamp within i64-ns range"))
    }
}

/// [`NaiveDate`] codec: a big-endian `i32` of days from the Common Era epoch (year 1 CE).
impl DecodeValue<&[u8]> for NaiveDate {
    fn decode_value(input: &mut &[u8]) -> crate::Result<Self> {
        let days = decb::be_i32(input)?;
        NaiveDate::from_num_days_from_ce_opt(days).ok_or_else(|| ContextError::from_input(input))
    }
}
impl EncodeValue<Vec<u8>> for NaiveDate {
    fn encode_value(&self) -> Vec<u8> {
        encb::be_i32(self.num_days_from_ce())
    }
}

/// [`NaiveTime`] codec: a big-endian `u32` of seconds from midnight (leap-second nanos dropped).
impl DecodeValue<&[u8]> for NaiveTime {
    fn decode_value(input: &mut &[u8]) -> crate::Result<Self> {
        let secs = decb::be_u32(input)?;
        NaiveTime::from_num_seconds_from_midnight_opt(secs, 0)
            .ok_or_else(|| ContextError::from_input(input))
    }
}
impl EncodeValue<Vec<u8>> for NaiveTime {
    fn encode_value(&self) -> Vec<u8> {
        encb::be_u32(self.num_seconds_from_midnight())
    }
}

/// [`Duration`] codec: a big-endian `u64` of nanoseconds.
impl DecodeValue<&[u8]> for Duration {
    fn decode_value(input: &mut &[u8]) -> crate::Result<Self> {
        Ok(Duration::from_nanos(decb::be_u64(input)?))
    }
}
impl EncodeValue<Vec<u8>> for Duration {
    fn encode_value(&self) -> Vec<u8> {
        // lossless: any `Duration::from_nanos(u64)` has `as_nanos() <= u64::MAX`
        encb::be_u64(self.as_nanos() as u64)
    }
}

/// [`Ipv4Addr`] codec: a big-endian `u32` (network-order octets).
impl DecodeValue<&[u8]> for Ipv4Addr {
    fn decode_value(input: &mut &[u8]) -> crate::Result<Self> {
        Ok(Ipv4Addr::from(decb::be_u32(input)?))
    }
}
impl EncodeValue<Vec<u8>> for Ipv4Addr {
    fn encode_value(&self) -> Vec<u8> {
        encb::be_u32(u32::from(*self))
    }
}

/// [`Ipv6Addr`] codec: a big-endian `u128` (network-order octets).
impl DecodeValue<&[u8]> for Ipv6Addr {
    fn decode_value(input: &mut &[u8]) -> crate::Result<Self> {
        Ok(Ipv6Addr::from(decb::be_u128(input)?))
    }
}
impl EncodeValue<Vec<u8>> for Ipv6Addr {
    fn encode_value(&self) -> Vec<u8> {
        encb::be_u128(u128::from(*self))
    }
}

/// [`char`] codec: a big-endian `u32` scalar value; decode rejects surrogates / out-of-range.
impl DecodeValue<&[u8]> for char {
    fn decode_value(input: &mut &[u8]) -> crate::Result<Self> {
        let scalar = decb::be_u32(input)?;
        char::try_from(scalar).map_err(|_| ContextError::from_input(input))
    }
}
impl EncodeValue<Vec<u8>> for char {
    fn encode_value(&self) -> Vec<u8> {
        encb::be_u32(u32::from(*self))
    }
}

/// [`NonZeroU32`] codec: a big-endian `u32`; decode rejects `0`.
impl DecodeValue<&[u8]> for NonZeroU32 {
    fn decode_value(input: &mut &[u8]) -> crate::Result<Self> {
        NonZeroU32::new(decb::be_u32(input)?).ok_or_else(|| ContextError::from_input(input))
    }
}
impl EncodeValue<Vec<u8>> for NonZeroU32 {
    fn encode_value(&self) -> Vec<u8> {
        encb::be_u32(self.get())
    }
}

/// [`bool`] codec: a single byte, `0` is false and any non-zero is true.
impl DecodeValue<&[u8]> for bool {
    fn decode_value(input: &mut &[u8]) -> crate::Result<Self> {
        Ok(decb::u8(input)? != 0)
    }
}
impl EncodeValue<Vec<u8>> for bool {
    fn encode_value(&self) -> Vec<u8> {
        encb::u8(u8::from(*self))
    }
}
