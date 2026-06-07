//! Native-Rust-type codecs
//!
//! Note: this is not yet folded into the crate, but could be later. For
//! now, gated behind the opt-in `bench` feature.
//!
//! Hand-written [`DecodeValue`](crate::DecodeValue) / [`EncodeValue`](crate::EncodeValue) impls
//! for common `std` / `chrono` types, so a derived KLV struct can use them as **first-class
//! fields** - `timestamp: DateTime<Utc>`, `addr: Ipv4Addr`, ... - and decode straight into them
//! with no per-field `dec`/`enc` attribute and no separate "raw" struct. This is only sound
//! *inside* tinyklv: the traits are ours, so impl-ing them for foreign types is coherence-legal
//! here (a downstream crate could not)
//!
//! Gated behind the opt-in `bench` feature; the name is provisional (this could be exposed as a
//! stable `native` surface one day). It exists today to drive the `NativeNested` benchmark record
//!
//! Design notes:
//! * **Endianness is big-endian by fiat.** A trait impl is one-wire-form-per-type, unlike the
//!   primitive codecs which parameterize endianness at the call site via
//!   `default(dec = be_*/le_*)` free functions. BE is chosen to match the rest of the suite
//! * **[`NaiveTime`] is lossy for leap seconds**: [`Timelike::num_seconds_from_midnight`] drops the
//!   leap nanosecond. Round-trips are exact only for non-leap times
//! * The [`bool`] / [`char`] impls are a permanent semantic commitment while the feature is on
//!   (`bool` = `u8 != 0`, `char` = big-endian `u32`); revisit with newtypes if `bench` ever
//!   stabilizes into a public API
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
use std::{
    net::{Ipv4Addr, Ipv6Addr},
    time::Duration,
};
use winnow::error::{ContextError, ParserError};

/// [`DateTime<Utc>`] implementation of [`DecodeValue`]
///
/// Decodes as a big-endian `i64` of nanoseconds since the Unix epoch
impl DecodeValue<&[u8]> for DateTime<Utc> {
    fn decode_value(input: &mut &[u8]) -> crate::Result<Self> {
        Ok(DateTime::from_timestamp_nanos(decb::be_i64(input)?))
    }
}
/// [`DateTime<Utc>`] implementation of [`EncodeValue`]
///
/// Encodes as a big-endian `i64` of nanoseconds since the Unix epoch
///
/// # Safety
///
/// Uses `expect` to unwrap [`chrono::DateTime::timestamp_nanos_opt`]; this panics
/// if the timestamp falls outside the `i64` nanosecond range (roughly ±292 years
/// from epoch). Timestamps produced by normal system clocks or the benchmark
/// fixtures are well within that range
impl EncodeValue for DateTime<Utc> {
    fn encode_value(&self, out: &mut Vec<u8>) {
        // infallible for the bounded timestamps `RngSample` emits (well inside the i64-ns range)
        encb::be_i64(
            self.timestamp_nanos_opt()
                .expect("timestamp within i64-ns range"),
            out,
        );
    }
}

/// [`NaiveDate`] implementation of [`DecodeValue`]
///
/// Decodes as a big-endian `i32` of days from the Common Era epoch (year 1 CE)
impl DecodeValue<&[u8]> for NaiveDate {
    fn decode_value(input: &mut &[u8]) -> crate::Result<Self> {
        let days = decb::be_i32(input)?;
        NaiveDate::from_num_days_from_ce_opt(days).ok_or_else(|| ContextError::from_input(input))
    }
}
/// [`NaiveDate`] implementation of [`EncodeValue`]
///
/// Encodes as a big-endian `i32` of days from the Common Era epoch (year 1 CE)
impl EncodeValue for NaiveDate {
    fn encode_value(&self, out: &mut Vec<u8>) {
        encb::be_i32(self.num_days_from_ce(), out);
    }
}

/// [`NaiveTime`] implementation of [`DecodeValue`]
///
/// Decodes as a big-endian `u32` of seconds from midnight; leap-second nanoseconds are dropped
impl DecodeValue<&[u8]> for NaiveTime {
    fn decode_value(input: &mut &[u8]) -> crate::Result<Self> {
        let secs = decb::be_u32(input)?;
        NaiveTime::from_num_seconds_from_midnight_opt(secs, 0)
            .ok_or(ContextError::from_input(input))
    }
}
/// [`NaiveTime`] implementation of [`EncodeValue`]
///
/// Encodes as a big-endian `u32` of seconds from midnight; leap-second
/// nanoseconds are dropped (see the module-level design note)
impl EncodeValue for NaiveTime {
    fn encode_value(&self, out: &mut Vec<u8>) {
        encb::be_u32(self.num_seconds_from_midnight(), out);
    }
}

/// [`Duration`] implementation of [`DecodeValue`]
///
/// Decodes as a big-endian `u64` of nanoseconds
impl DecodeValue<&[u8]> for Duration {
    fn decode_value(input: &mut &[u8]) -> crate::Result<Self> {
        Ok(Duration::from_nanos(decb::be_u64(input)?))
    }
}
/// [`Duration`] implementation of [`EncodeValue`]
///
/// Encodes as a big-endian `u64` of nanoseconds; the cast is lossless because
/// any [`Duration`] created from `u64` nanoseconds has `as_nanos() <= u64::MAX`
impl EncodeValue for Duration {
    fn encode_value(&self, out: &mut Vec<u8>) {
        // lossless: any `Duration::from_nanos(u64)` has `as_nanos() <= u64::MAX`
        encb::be_u64(self.as_nanos() as u64, out);
    }
}

/// [`Ipv4Addr`] implementation of [`DecodeValue`]
///
/// Decodes as a big-endian `u32` in network byte order
impl DecodeValue<&[u8]> for Ipv4Addr {
    fn decode_value(input: &mut &[u8]) -> crate::Result<Self> {
        Ok(Ipv4Addr::from(decb::be_u32(input)?))
    }
}
/// [`Ipv4Addr`] implementation of [`EncodeValue`]
///
/// Encodes as a big-endian `u32` in network byte order (same as `u32::from(addr).to_be_bytes()`)
impl EncodeValue for Ipv4Addr {
    fn encode_value(&self, out: &mut Vec<u8>) {
        encb::be_u32(u32::from(*self), out);
    }
}

/// [`Ipv6Addr`] implementation of [`DecodeValue`]
///
/// Decodes as a big-endian `u128` in network byte order
impl DecodeValue<&[u8]> for Ipv6Addr {
    fn decode_value(input: &mut &[u8]) -> crate::Result<Self> {
        Ok(Ipv6Addr::from(decb::be_u128(input)?))
    }
}
/// [`Ipv6Addr`] implementation of [`EncodeValue`]
///
/// Encodes as a big-endian `u128` in network byte order (same as `u128::from(addr).to_be_bytes()`)
impl EncodeValue for Ipv6Addr {
    fn encode_value(&self, out: &mut Vec<u8>) {
        encb::be_u128(u128::from(*self), out);
    }
}

/// [`char`] implementation of [`DecodeValue`]
///
/// Decodes as a big-endian `u32` Unicode scalar value; rejects surrogates and out-of-range values
impl DecodeValue<&[u8]> for char {
    fn decode_value(input: &mut &[u8]) -> crate::Result<Self> {
        let scalar = decb::be_u32(input)?;
        char::try_from(scalar).map_err(|_| ContextError::from_input(input))
    }
}
/// [`char`] implementation of [`EncodeValue`]
///
/// Encodes as a big-endian `u32` Unicode scalar value
impl EncodeValue for char {
    fn encode_value(&self, out: &mut Vec<u8>) {
        encb::be_u32(u32::from(*self), out);
    }
}

/// [`NonZeroU32`] implementation of [`DecodeValue`]
///
/// Decodes as a big-endian `u32`; returns an error if the decoded value is `0`
impl DecodeValue<&[u8]> for NonZeroU32 {
    fn decode_value(input: &mut &[u8]) -> crate::Result<Self> {
        NonZeroU32::new(decb::be_u32(input)?).ok_or_else(|| ContextError::from_input(input))
    }
}
/// [`NonZeroU32`] implementation of [`EncodeValue`]
///
/// Encodes the wrapped non-zero value as a big-endian `u32`
impl EncodeValue for NonZeroU32 {
    fn encode_value(&self, out: &mut Vec<u8>) {
        encb::be_u32(self.get(), out);
    }
}

/// [`bool`] implementation of [`DecodeValue`]
///
/// Decodes a single byte: `0` maps to `false`, any non-zero value maps to `true`
impl DecodeValue<&[u8]> for bool {
    fn decode_value(input: &mut &[u8]) -> crate::Result<Self> {
        Ok(decb::u8(input)? != 0)
    }
}
/// [`bool`] implementation of [`EncodeValue`]
///
/// Encodes as a single byte: `0x00` for `false`, `0x01` for `true`
impl EncodeValue for bool {
    fn encode_value(&self, out: &mut Vec<u8>) {
        encb::u8(u8::from(*self), out);
    }
}
