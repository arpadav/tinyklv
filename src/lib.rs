#![doc = include_str!("../README.md")]
//! Author: aav
// --------------------------------------------------
// mods
// --------------------------------------------------
pub mod codecs;
pub mod decoder;
pub mod framing;
pub mod traits;

// --------------------------------------------------
// re-exports
// --------------------------------------------------
pub use codecs::*;
pub use decoder::{Decoder, Packet};
pub use framing::Framing;
pub use tinyklv_impl::*;
pub use traits::*;

#[doc(hidden)]
/// Internal re-exports and helpers used by `#[derive(Klv)]` expansions
///
/// Not part of the public API - may change without notice
pub mod __export {
    #[cfg(feature = "chrono")]
    pub use chrono;
    pub use memchr;
    pub use winnow;

    /// Builds a [`winnow::error::ContextError`] anchored at `input` carrying a
    /// single [`winnow::error::StrContext::Label`]
    ///
    /// Shared construction path for errors that cannot be expressed through
    /// a parser combinator (precondition failures and derive-generated
    /// decoders); a site that needs more context adds it to the returned
    /// error. Keeps manual error construction uniform with the hand-written
    /// codecs
    ///
    /// Not part of the public API - may change without notice
    ///
    /// # Arguments
    ///
    /// * `input` - the stream the error is anchored to
    /// * `checkpoint` - the position recorded before the failing step
    /// * `label` - a static description of what was being parsed
    ///
    /// # Returns
    ///
    /// A fresh error whose only context is `label`
    #[must_use]
    pub fn labeled_error<S>(
        input: &S,
        checkpoint: &<S as winnow::stream::Stream>::Checkpoint,
        label: &'static str,
    ) -> winnow::error::ContextError
    where
        S: winnow::stream::Stream,
    {
        use winnow::error::{AddContext, ParserError};
        winnow::error::ContextError::from_input(input).add_context(
            input,
            checkpoint,
            winnow::error::StrContext::Label(label),
        )
    }

    /// Names the site of a failed parse in `err`, so that its display shows `label` first, then
    /// why the parser failed
    ///
    /// [`winnow::error::ContextError`]'s display names only its first
    /// [`winnow::error::StrContext::Label`]. An `err` without a label gains `label` in place,
    /// keeping its context and cause. An `err` with one becomes the cause of a new error labelled
    /// `label`, then `err`'s first label, whose display prints `label`, then `err`'s own
    /// display. Neither path copies more than one label of `err`, so relabelling a value nested
    /// `n` deep costs `O(n)`
    ///
    /// Not part of the public API - may change without notice
    ///
    /// # Arguments
    ///
    /// * `input` - the stream the error is anchored to
    /// * `checkpoint` - the position recorded before the failing step
    /// * `label` - a static description of what was being parsed
    /// * `err` - the error of the parser that failed there
    ///
    /// # Returns
    ///
    /// `err` with `label` added, when it carried no label of its own; otherwise a new error
    /// labelled `label`, then `err`'s first label, with `err` as its cause
    #[must_use]
    pub fn relabeled_error<S>(
        input: &S,
        checkpoint: &<S as winnow::stream::Stream>::Checkpoint,
        label: &'static str,
        err: winnow::error::ContextError,
    ) -> winnow::error::ContextError
    where
        S: winnow::stream::Stream,
    {
        use winnow::error::{AddContext, ContextError, FromExternalError, StrContext};
        // --------------------------------------------------
        // the first label `err` already carries, if any
        // --------------------------------------------------
        let own_label = err
            .context()
            .find(|context| matches!(context, StrContext::Label(_)))
            .cloned();
        // --------------------------------------------------
        // without a label of its own, `err` gains `label` in place
        // --------------------------------------------------
        let Some(own_label) = own_label else {
            return err.add_context(input, checkpoint, StrContext::Label(label));
        };
        // --------------------------------------------------
        // with one, `err` becomes the cause of a new error labelled `label`
        // --------------------------------------------------
        let mut relabeled =
            <ContextError as FromExternalError<S, LabeledCause>>::from_external_error(
                input,
                LabeledCause(err),
            )
            .add_context(input, checkpoint, StrContext::Label(label));
        // --------------------------------------------------
        // `err`'s own first label follows `label`
        // --------------------------------------------------
        relabeled.push(own_label);
        relabeled
    }

    /// A labelled parser error that [`relabeled_error`] carries as the cause of the error it
    /// builds, displayed as the parser's own error
    #[derive(Debug)]
    struct LabeledCause(winnow::error::ContextError);

    /// [`LabeledCause`] implementation of [`std::fmt::Display`]
    impl std::fmt::Display for LabeledCause {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            std::fmt::Display::fmt(&self.0, f)
        }
    }

    /// [`LabeledCause`] implementation of [`std::error::Error`]
    ///
    /// Its `source` is the parser's own cause, so a typed cause stays reachable through the
    /// chain. A nested [`LabeledCause`] is skipped, since this display already prints it
    impl std::error::Error for LabeledCause {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            let mut cause = self.0.cause()?;
            while let Some(LabeledCause(inner)) = cause.downcast_ref::<LabeledCause>() {
                cause = inner.cause()?;
            }
            Some(cause)
        }
    }
}

/// Convenience re-export of all traits and parser primitives needed to work with KLV streams
///
/// Importing `tinyklv::prelude::*` brings into scope the [`Decoder`],
/// [`Packet`], all codec traits ([`DecodeValue`], [`EncodeValue`],
/// [`EncodeFrame`], [`DecodeFrame`]), the [`BreakType`] loop-control enum, the
/// [`Framing`] fused-`keylen` frame descriptor, the [`Klv`] derive macro, and
/// the winnow combinators and stream utilities that the derive-generated code
/// depends on
pub mod prelude {
    // --------------------------------------------------
    // local
    // --------------------------------------------------
    pub use crate::{
        decoder::{Decoder, Packet},
        framing::Framing,
        traits::{
            BreakType, DecodeFrame as _, DecodePartial, DecodeValue, DrainFrames as _, EncodeAs,
            EncodeFrame as _, EncodeValue, Partial, SeekSentinel as _,
        },
    };
    pub use tinyklv_impl::Klv;
    // --------------------------------------------------
    // external
    // --------------------------------------------------
    pub use winnow::{Parser as _, error::AddContext as _, prelude::*, stream::Stream as _};
}

/// Convenience type alias for [`winnow::Result`] used throughout the tinyklv codec API
///
/// Equivalent to `Result<T, winnow::error::ContextError>`. All decoder
/// functions in `tinyklv` return this type so that they compose directly
/// with winnow combinators without extra type annotations
pub type Result<T> = winnow::Result<T>;

#[macro_export]
/// Decodes a field and multiplies the result by a scale factor, yielding a floating-point value
///
/// Applies `(parser(input)? as $precision) * $scale`, casting the raw decoded
/// integer to the target floating-point type before multiplying. The result is
/// a closure compatible with `#[klv(dec = ...)]`
///
/// Typical use: a raw integer encodes a physical quantity at a known LSB
/// resolution (e.g. `360.0 / 65535.0` degrees per count)
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
/// Decodes a field and casts the result to a target type without scaling
///
/// Applies `parser(input)? as $precision`, which performs a lossless or
/// narrowing numeric cast depending on the types involved. The result is
/// a closure compatible with `#[klv(dec = ...)]`
///
/// Use this when the raw decoded integer already represents the desired
/// value without any unit conversion. For scaled values, use [`scale!`]
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
/// Encodes a floating-point field by dividing by a scale factor, casting to the wire type, then encoding
///
/// Applies `encoder((*input / $scale) as $data)`, which is the exact inverse
/// of [`scale!`]. The result is a closure compatible with `#[klv(enc = ...)]`
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
/// let mut encoded = Vec::new();
/// encoder(&3.0_f64, &mut encoded);
/// assert_eq!(encoded, vec![0x00, 0x01]);
/// ```
macro_rules! scale_enc {
    ($encoder:path, $precision:ty, $data:ty, $scale:tt $(,)*) => {
        |input: &$precision, out: &mut Vec<u8>| $encoder((*input / $scale) as $data, out)
    };
}

#[macro_export]
/// Encodes a field by casting to the wire type and encoding, without applying any scale factor
///
/// Applies `encoder(*input as $data)`, which is the direct inverse of
/// [`cast!`]. The result is a closure compatible with `#[klv(enc = ...)]`
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
/// let mut encoded = Vec::new();
/// encoder(&1.0_f64, &mut encoded);
/// assert_eq!(encoded, vec![0x00, 0x01]);
/// ```
macro_rules! cast_enc {
    ($encoder:path, $precision:ty, $data:ty $(,)*) => {
        |input: &$precision, out: &mut Vec<u8>| $encoder(*input as $data, out)
    };
}

#[macro_export]
/// Encodes a field by subtracting an offset, dividing by a scale factor, casting to the wire type, then encoding
///
/// Applies `encoder(((*input - $offset) / $scale) as $data)`, which encodes
/// real-world values that are mapped to a data range via
/// `data_value = (real_value - offset) / scale`. This is the encode-side
/// counterpart to a decode pipeline that uses [`scale!`] combined with a
/// manual offset addition
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
/// let mut encoded = Vec::new();
/// // (14.0 - 10.0) / 2.0 = 2.0 as u16 = 2
/// encoder(&14.0_f64, &mut encoded);
/// assert_eq!(encoded, vec![0x00, 0x02]);
/// ```
macro_rules! scale_offset_enc {
    ($encoder:path, $precision:ty, $data:ty, $scale:tt, $offset:tt $(,)*) => {
        |input: &$precision, out: &mut Vec<u8>| {
            $encoder(((*input - $offset) / $scale) as $data, out)
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
/// let date = tinyklv::as_date!(tinyklv::dec::string::to_string_utf8, "%Y-%m-%d", len)(input);
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
/// let time = tinyklv::as_time!(tinyklv::dec::string::to_string_utf8, "%H:%M:%S", 8)(input);
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
/// let datetime = tinyklv::as_datetime!(tinyklv::dec::string::to_string_utf8, "%Y-%m-%d %H:%M:%S", input.len())(input);
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
