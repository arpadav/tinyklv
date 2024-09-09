#![doc = include_str!("../README.md")]
pub mod _tutorial;
pub mod prelude;
pub mod codecs;
pub use codecs::*;
pub mod reexport {
    pub use winnow;
}
pub use tinyklv_impl::*;

pub(crate) fn enc_prior<T, const N: usize>(input: Option<T>, enc: fn(T) -> [u8; N]) -> Vec<u8> {
    // Some: convert resulting [u8; N] -> Vec<u8>
    // None: return an empty Vec<u8>
    input.map(|i| enc(i).into())
    .unwrap_or_default()
}

#[macro_export]
/// Returns a blank context error: usually used for reserved values.
/// 
/// It is not recommended to use this unless a [`None`] value has to be
/// returned upon parsing values.
macro_rules! err {
    () => { winnow::error::ErrMode::Backtrack(winnow::error::ContextError::new()) };
}

// #[macro_export]
// /// Perform some operation on a parsed value of some defined precision
// /// 
// /// # Example
// /// 
// /// ```rust no_run ignore
// /// tinyklv::op!(input, parser, f64, * 100.0, - 10.0);
// /// // expands to:
// /// let _ = (parser.parse_next(input)? * 100.0) - 10.0;
// /// 
// /// tinyklv::op!(input, parser, f64, * 100.0, - 10.0, + 12.0, / 2.0, + 1.0);
// /// // expands to:
// /// let _ = (((parser.parse_next(input)? * 100.0) - 10.0 + 12.0) / 2.0) + 1.0;
// /// ```
// macro_rules! op {
//     // Base case: single operation
//     ($input:tt, $parser:path, $precision:ty, $op:tt $val:expr) => {
//         $parser.parse_next($input)? $op $val
//     };

//     // Recursive case: multiple operations
//     ($input:tt, $parser:path, $precision:ty, $op1:tt $val1:expr, $($op2:tt $val2:expr),*) => {
//         $crate::op!(@apply $input, $parser, $precision, $op1 $val1, $($op2 $val2),*)
//     };

//     // Helper to apply remaining operations
//     (@apply $input:tt, $parser:path, $precision:ty, $op1:tt $val1:expr) => {
//         $parser.parse_next($input)? $op1 $val1
//     };

//     (@apply $input:tt, $parser:path, $precision:ty, $op1:tt $val1:expr, $($op2:tt $val2:expr),*) => {
//         $crate::op!(@apply $input, $parser, $precision, ($parser.parse_next($input)? as $precision $op1 $val1) $($op2 $val2),*)
//     };
// }
// Example Usage
// tinyklv::op!(input, parser, f64, * 100.0, - 10.0, + 12.0, / 2.0, + 1.0);


#[macro_export]
/// Scales a parsed value of some predefined precision
/// 
/// Can be used directly in a `#[klv(dec = ...)]` attribute
/// 
/// # Example
/// 
/// ```rust no_run ignore
/// tinyklv::scale!(tinyklv::codecs::binary::dec::be_u16, f64, KLV_2_PLATFORM_HEADING)(input)
/// // OR
/// #[klv(dec = tinyklv::scale!(tinyklv::codecs::binary::dec::be_u16, f64, KLV_2_PLATFORM_HEADING))]
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
/// # Example
/// 
/// ```rust no_run ignore
/// tinyklv::cast!(tinyklv::codecs::binary::dec::be_u16, f64)(input)
/// // OR
/// #[klv(dec = tinyklv::cast!(tinyklv::codecs::binary::dec::be_u16, f64))]
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
/// let date = tinyklv::as_date!(tinyklv::dec::binary::to_string_utf8, "%Y-%m-%d")(input, len);
/// assert_eq!(date, Ok(chrono::NaiveDate::from_ymd_opt(2020, 12, 31).unwrap()));
/// ```
macro_rules! as_date {
    ($str_parser:path, $date_fmt:tt $(,)*) => {
        |input, len| -> winnow::PResult<chrono::NaiveDate> {
            chrono::NaiveDate::parse_from_str(
                &$str_parser(input, len)?,
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
/// let len = 8;
/// let time = tinyklv::as_time!(tinyklv::dec::binary::to_string_utf8, "%H:%M:%S")(input, len);
/// assert_eq!(time, Ok(chrono::NaiveTime::from_hms_opt(12, 34, 56).unwrap()));
/// ```
macro_rules! as_time {
    ($str_parser:path, $time_fmt:tt $(,)*) => {
        |input, len| -> winnow::PResult<chrono::NaiveTime> {
            chrono::NaiveTime::parse_from_str(
                &$str_parser(input, len)?,
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
/// let len = 19;
/// let datetime = tinyklv::as_datetime!(tinyklv::dec::binary::to_string_utf8, "%Y-%m-%d %H:%M:%S")(input, len);
/// assert_eq!(datetime, Ok(chrono::NaiveDate::from_ymd_opt(2020, 12, 31).unwrap().and_hms_opt(12, 34, 56).unwrap()));
/// ```
macro_rules! as_datetime {
    ($str_parser:path, $datetime_fmt:tt $(,)*) => {
        |input, len| -> winnow::PResult<chrono::NaiveDateTime> {
            chrono::NaiveDateTime::parse_from_str(
                &$str_parser(input, len)?,
                $datetime_fmt,
            ).map_err(|_| tinyklv::err!())
        }
    };
}