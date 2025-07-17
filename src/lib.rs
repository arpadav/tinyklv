#![doc = include_str!("../README.md")]
// --------------------------------------------------
// mods
// --------------------------------------------------
pub mod _tutorial;
pub mod traits;
pub mod codecs;

// --------------------------------------------------
// local
// --------------------------------------------------
pub use codecs::*;
pub use traits::*;
pub use tinyklv_impl::*;

// --------------------------------------------------
// internal re-exports: used during macro expansion
// --------------------------------------------------
pub mod __export {
    pub use winnow;
    pub use memchr;
    #[cfg(feature = "chrono")]
    pub use chrono;
}

pub mod prelude {
    // --------------------------------------------------
    // external
    // --------------------------------------------------
    pub use winnow::prelude::*;
    pub use winnow::Parser as _;
    pub use winnow::stream::Stream as _;
    pub use winnow::error::AddContext as _;
    // --------------------------------------------------
    // local
    // --------------------------------------------------
    pub use crate::traits::Seek as _;
    pub use crate::traits::Decode as _;
    pub use crate::traits::Extract as _;
    pub use crate::traits::RepeatedDecode as _;

    pub use crate::traits::Encode as _;
    pub use crate::traits::IntoKlv as _;
    pub use crate::traits::EncodeValue as _;
    
    pub use crate::traits::BreakCondition as _;
    // pub use crate::traits::BreakConditionType as _;
}

pub type Result<T> = winnow::Result<T>;

#[deprecated]
#[macro_export]
/// Returns a blank, unrecoverable error.
/// 
/// This is helpful for quick development. However, **it is not recommended
/// to use this** since the error is non-descriptive.
macro_rules! err2 {
    () => {
        winnow::error::ContextError::new()
    };

    ($input:ident, $checkpoint:ident, $msg:expr) => {
        winnow::error::ErrMode::Cut(winnow::error::ContextError::new().add_context(
            $input,
            &$checkpoint,
            winnow::error::StrContext::Label($msg),
        ))
    };
}

#[macro_export]
/// Returns a blank, unrecoverable error.
macro_rules! err {
    ($input:ident, $checkpoint:ident, $msg:expr) => {
        Err(winnow::error::ContextError::new().add_context(
            $input,
            &$checkpoint,
            winnow::error::StrContext::Label($msg),
        ))
    };

    ($err:ident, $input:ident, $checkpoint:ident, $msg:expr) => {
        Err($err.add_context(
            $input,
            &$checkpoint,
            winnow::error::StrContext::Label($msg),
        ))
    };
}

#[macro_export]
/// Returns a blank, unrecoverable error.
macro_rules! ctxt {
    ($input:ident, $checkpoint:ident, $msg:expr) => {
        winnow::error::ContextError::new().add_context(
            $input,
            &$checkpoint,
            winnow::error::StrContext::Label($msg),
        )
    };

    ($err:ident, $input:ident, $checkpoint:ident, $msg:expr) => {
        $err.add_context(
            $input,
            &$checkpoint,
            winnow::error::StrContext::Label($msg),
        )
    };
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
        |input| -> ::tinyklv::Result<$precision> {
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
        |input| -> ::tinyklv::Result<::tinyklv::__export::chrono::NaiveDate> {
            ::tinyklv::__export::chrono::NaiveDate::parse_from_str(
                &$str_parser($len)(input)?,
                $date_fmt,
            ).map_err(|_| ::tinyklv::err!())
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
            ::tinyklv::__export::chrono::NaiveTime::parse_from_str(
                &$str_parser($len)(input)?,
                $time_fmt,
            ).map_err(|_| ::tinyklv::err!())
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
            ::tinyklv::__export::chrono::NaiveDateTime::parse_from_str(
                &$str_parser($len)(input)?,
                $datetime_fmt,
            ).map_err(|_| tinyklv::err!())
        }
    };
}