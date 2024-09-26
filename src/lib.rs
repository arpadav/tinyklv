#![doc = include_str!("../README.md")]
pub mod _tutorial;
pub mod prelude;
pub mod codecs;
pub use codecs::*;
pub mod reexport {
    pub use winnow;
}
pub use tinyklv_impl::*;

#[macro_export]
/// Returns a blank context error: usually used for reserved values.
/// 
/// It is not recommended to use this unless a [`None`] value has to be
/// returned upon parsing values.
macro_rules! err {
    () => { winnow::error::ErrMode::Backtrack(winnow::error::ContextError::new()) };
}

#[macro_export]
/// Scales a parsed value of some predefined precision
/// 
/// Can be used directly in a `#[klv(dec = ...)]` attribute
/// 
/// # Usage
/// 
/// ```rust ignore
/// tinyklv::scale!(tinyklv::codecs::binary::dec::be_u16, f64, KLV_2_PLATFORM_HEADING)(input)
/// // OR
/// #[klv(dec = tinyklv::scale!(tinyklv::codecs::binary::dec::be_u16, f64, KLV_2_PLATFORM_HEADING))]
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
        |input| -> winnow::PResult<$precision> {
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
/// ```rust ignore
/// tinyklv::cast!(tinyklv::codecs::binary::dec::be_u16, f64)(input)
/// // OR
/// #[klv(dec = tinyklv::cast!(tinyklv::codecs::binary::dec::be_u16, f64))]
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
        |input| -> winnow::PResult<$precision> {
            Ok($parser.parse_next(input)? as $precision)
        }
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
        |input| -> winnow::PResult<chrono::NaiveDate> {
            chrono::NaiveDate::parse_from_str(
                &$str_parser($len)(input)?,
                $date_fmt,
            ).map_err(|_| tinyklv::err!())
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
        |input| -> winnow::PResult<chrono::NaiveTime> {
            chrono::NaiveTime::parse_from_str(
                &$str_parser($len)(input)?,
                $time_fmt,
            ).map_err(|_| tinyklv::err!())
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
        |input| -> winnow::PResult<chrono::NaiveDateTime> {
            chrono::NaiveDateTime::parse_from_str(
                &$str_parser($len)(input)?,
                $datetime_fmt,
            ).map_err(|_| tinyklv::err!())
        }
    };
}