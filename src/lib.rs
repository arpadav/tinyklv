#![doc = include_str!("../README.md")]
//! Author: aav
// --------------------------------------------------
// mods
// --------------------------------------------------
pub mod codecs;
pub mod traits;
// --------------------------------------------------
// re-exports
// --------------------------------------------------
pub use codecs::*;
pub use tinyklv_impl::*;
pub use traits::*;

#[doc(hidden)]
/// Internal re-exports used during proc-macro expansion.
/// Not part of the public API - may change without notice.
pub mod __export {
    #[cfg(feature = "chrono")]
    pub use chrono;
    pub use memchr;
    pub use winnow;
}

/// Wraps an owned-taking encoder for use in `#[klv(enc = ...)]`
///
/// The derive macro passes `&T` to encoder functions. If your encoder takes
/// an owned `T` instead, wrap it with this macro:
///
/// ```rust,ignore
/// #[klv(key = 0x01, dec = decode_color, enc = tinyklv::enc_owned!(encode_color))]
/// ```
///
/// Requires `T: Clone`. Equivalent to `|v| encoder(Clone::clone(v))`.
#[macro_export]
macro_rules! enc_owned {
    ($encoder:path) => {
        |v| $encoder(::core::clone::Clone::clone(v))
    };
}

/// Convenience re-export of all traits and parser primitives needed to
/// work with KLV streams
///
/// Import this module with `use tinyklv::prelude::*` to bring all decode
/// and encode traits, the winnow parser primitives, and break-condition
/// types into scope
pub mod prelude {
    // --------------------------------------------------
    // local
    // --------------------------------------------------
    pub use crate::traits::{
        BreakCondition as _, BreakConditionType, DecodeFrame as _, DecodeValue as _,
        EncodeFrame as _, EncodeValue as _, EncodedOutput as _, IntoKlv as _, RepeatedDecode as _,
        SeekSentinel as _,
    };
    // --------------------------------------------------
    // external
    // --------------------------------------------------
    pub use winnow::{error::AddContext as _, prelude::*, stream::Stream as _, Parser as _};
}

pub type Result<T> = winnow::Result<T>;

#[macro_export]
/// Scales a parsed value of some predefined precision
///
/// Can be used directly in a `#[klv(dec = ...)]` attribute
///
/// # Usage
///
/// ```rust
/// use tinyklv::prelude::*;
///
/// const TELEMETRY_DATA: f64 = 360.0 / 65535.0;
/// let mut input: &[u8] = &[0x00, 0x01];
/// let input = &mut input;
/// let _ = tinyklv::scale!(tinyklv::codecs::binary::dec::be_u16, f64, TELEMETRY_DATA)(input);
/// // Within a derive macro:
/// // #[klv(dec = tinyklv::scale!(tinyklv::codecs::binary::dec::be_u16, f64, TELEMETRY_DATA))]
/// ```
///
/// # Example
///
/// ```rust
/// use tinyklv::prelude::*;
///
/// let mut input: &[u8] = &[0x00, 0x01];
/// let input = &mut input;
/// let num = tinyklv::scale!(tinyklv::codecs::binary::dec::be_u16, f32, 3.0)(input);
/// assert_eq!(num, Ok(3.0_f32));
/// ```
macro_rules! scale {
    ($parser:path, $precision:ty, $scale:tt $(,)*) => {
        |input| -> ::tinyklv::Result<$precision> {
            Ok(($parser.parse_next(input)? as $precision) * $scale)
        }
    };
}

#[macro_export]
/// Sets precision of a parsed value
///
/// Can be used directly in a `#[klv(dec = ...)]` attribute
///
/// # Usage
///
/// ```rust
/// use tinyklv::prelude::*;
///
/// let mut input: &[u8] = &[0x00, 0x01];
/// let input = &mut input;
/// let _ = tinyklv::cast!(tinyklv::codecs::binary::dec::be_u16, f64)(input);
/// // Within a derive macro:
/// // #[klv(dec = tinyklv::cast!(tinyklv::codecs::binary::dec::be_u16, f64))]
/// ```
///
/// # Example
///
/// ```rust
/// use tinyklv::prelude::*;
///
/// let mut input: &[u8] = &[0x00, 0x01];
/// let input = &mut input;
/// let num = tinyklv::cast!(tinyklv::codecs::binary::dec::be_u16, f64)(input);
/// assert_eq!(num, Ok(1.0_f64));
/// ```
macro_rules! cast {
    ($parser:expr, $precision:ty $(,)*) => {
        |input| -> ::tinyklv::Result<$precision> { Ok($parser.parse_next(input)? as $precision) }
    };
}

#[macro_export]
/// Encode counterpart of [`scale!`]. Divides by scale factor, casts to wire type, then encodes.
///
/// Can be used directly in a `#[klv(enc = ...)]` attribute
///
/// # Usage
///
/// ```rust ignore
/// #[klv(enc = tinyklv::scale_enc!(tinyklv::codecs::binary::enc::be_u16, f64, u16, SCALE_FACTOR))]
/// ```
///
/// # Example
///
/// ```rust
/// let encoder = tinyklv::scale_enc!(tinyklv::codecs::binary::enc::be_u16, f64, u16, 3.0);
/// let encoded = encoder(&3.0_f64);
/// assert_eq!(encoded, vec![0x00, 0x01]);
/// ```
macro_rules! scale_enc {
    ($encoder:path, $precision:ty, $wire:ty, $scale:tt $(,)*) => {
        |input: &$precision| -> Vec<u8> { $encoder((*input / $scale) as $wire) }
    };
}

#[macro_export]
/// Encode counterpart of [`cast!`]. Casts to wire type, then encodes.
///
/// Can be used directly in a `#[klv(enc = ...)]` attribute
///
/// # Usage
///
/// ```rust ignore
/// #[klv(enc = tinyklv::cast_enc!(tinyklv::codecs::binary::enc::be_u16, f64, u16))]
/// ```
///
/// # Example
///
/// ```rust
/// let encoder = tinyklv::cast_enc!(tinyklv::codecs::binary::enc::be_u16, f64, u16);
/// let encoded = encoder(&1.0_f64);
/// assert_eq!(encoded, vec![0x00, 0x01]);
/// ```
macro_rules! cast_enc {
    ($encoder:path, $precision:ty, $wire:ty $(,)*) => {
        |input: &$precision| -> Vec<u8> { $encoder(*input as $wire) }
    };
}

#[macro_export]
/// Encode counterpart of [`scale!`] with offset. Subtracts offset, divides by scale, casts to wire type, then encodes.
///
/// Useful fields that map a real-value range to a wire-value range
/// via `wire_value = (real_value - offset) / scale`.
///
/// # Usage
///
/// ```rust ignore
/// #[klv(enc = tinyklv::scale_offset_enc!(tinyklv::codecs::binary::enc::be_u32, f64, u32, SCALE, OFFSET))]
/// ```
///
/// # Example
///
/// ```rust
/// let encoder = tinyklv::scale_offset_enc!(tinyklv::codecs::binary::enc::be_u16, f64, u16, 2.0, 10.0);
/// let encoded = encoder(&14.0_f64);
/// // (14.0 - 10.0) / 2.0 = 2.0 as u16 = 2
/// assert_eq!(encoded, vec![0x00, 0x02]);
/// ```
macro_rules! scale_offset_enc {
    ($encoder:path, $precision:ty, $wire:ty, $scale:tt, $offset:tt $(,)*) => {
        |input: &$precision| -> Vec<u8> { $encoder(((*input - $offset) / $scale) as $wire) }
    };
}

#[macro_export]
#[cfg(feature = "chrono")]
/// Parses a string as a date, using [`chrono::NaiveDate::parse_from_str`]
///
/// Can be used directly in a `#[klv(dec = ...)]` attribute
///
/// # Example
///
/// ```rust
/// use tinyklv::prelude::*;
///
/// let mut input: &[u8] = b"2020-12-31";
/// let input = &mut input;
/// let len = 10;
/// let date = tinyklv::as_date!(tinyklv::dec::binary::to_string_utf8, "%Y-%m-%d", len)(input);
/// assert_eq!(date, Ok(chrono::NaiveDate::from_ymd_opt(2020, 12, 31).unwrap()));
/// ```
macro_rules! as_date {
    ($str_parser:path, $date_fmt:tt, $len:expr $(,)*) => {
        |input| -> ::tinyklv::Result<::tinyklv::__export::chrono::NaiveDate> {
            $str_parser($len)
                .try_map(|s| ::tinyklv::__export::chrono::NaiveDate::parse_from_str(&s, $date_fmt))
                .context(::tinyklv::__export::winnow::error::StrContext::Label(
                    "date",
                ))
                .parse_next(input)
        }
    };
}

#[macro_export]
#[cfg(feature = "chrono")]
/// Parses a string as a time, using [`chrono::NaiveTime::parse_from_str`]
///
/// Can be used directly in a `#[klv(dec = ...)]` attribute
///
/// # Example
///
/// ```rust
/// use tinyklv::prelude::*;
///
/// let mut input: &[u8] = b"12:34:56";
/// let input = &mut input;
/// let time = tinyklv::as_time!(tinyklv::dec::binary::to_string_utf8, "%H:%M:%S", 8)(input);
/// assert_eq!(time, Ok(chrono::NaiveTime::from_hms_opt(12, 34, 56).unwrap()));
/// ```
macro_rules! as_time {
    ($str_parser:path, $time_fmt:tt, $len:expr $(,)*) => {
        |input| -> ::tinyklv::Result<::tinyklv::__export::chrono::NaiveTime> {
            $str_parser($len)
                .try_map(|s| ::tinyklv::__export::chrono::NaiveTime::parse_from_str(&s, $time_fmt))
                .context(::tinyklv::__export::winnow::error::StrContext::Label(
                    "time",
                ))
                .parse_next(input)
        }
    };
}

#[macro_export]
#[cfg(feature = "chrono")]
/// Parses a string as a datetime, using [`chrono::NaiveDateTime::parse_from_str`]
///
/// Can be used directly in a `#[klv(dec = ...)]` attribute
///
/// # Example
///
/// ```rust
/// use std::str::FromStr;
/// use tinyklv::prelude::*;
///
/// let mut input: &[u8] = b"2020-12-31 12:34:56";
/// let input = &mut input;
/// let datetime = tinyklv::as_datetime!(tinyklv::dec::binary::to_string_utf8, "%Y-%m-%d %H:%M:%S", input.len())(input);
/// assert_eq!(datetime, Ok(chrono::NaiveDate::from_ymd_opt(2020, 12, 31).unwrap().and_hms_opt(12, 34, 56).unwrap()));
/// ```
macro_rules! as_datetime {
    ($str_parser:path, $datetime_fmt:tt, $len:expr $(,)*) => {
        |input| -> ::tinyklv::Result<::tinyklv::__export::chrono::NaiveDateTime> {
            $str_parser($len)
                .try_map(|s| {
                    ::tinyklv::__export::chrono::NaiveDateTime::parse_from_str(&s, $datetime_fmt)
                })
                .context(::tinyklv::__export::winnow::error::StrContext::Label(
                    "datetime",
                ))
                .parse_next(input)
        }
    };
}
