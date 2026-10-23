#![doc = include_str!("../README.md")]
//! Author: aav
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// --------------------------------------------------
// mods
// --------------------------------------------------
pub mod codecs;
pub mod decoder;
mod error;
pub mod framing;
pub mod traits;

// --------------------------------------------------
// re-exports
// --------------------------------------------------
pub use codecs::{dec, enc};
pub use decoder::{Decoder, Packet};
pub use error::ErrorKind;
pub use framing::Framing;
pub use tinyklv_impl::Klv;
pub use traits::{
    BreakType, DecodeFrame, DecodePartial, DecodeValue, DrainFrames, EncodeAs, EncodeFrame,
    EncodeValue, Partial, PartialIterator, ResumePartial, SeekSentinel,
};
/// The `winnow` every codec signature is written in, so a custom decoder names the same
/// [`winnow::error::ContextError`] and [`winnow::Parser`] without a `winnow` dependency of its own
pub use winnow;

#[doc(hidden)]
/// Internal re-exports and helpers used by `#[derive(Klv)]` expansions
///
/// Not part of the public API - may change without notice
pub mod __export {
    #[cfg(feature = "chrono")]
    pub use chrono;
    pub use memchr;
    #[cfg(feature = "tracing")]
    pub use tracing;
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

    /// [`labeled_error`] tagged with `kind`, for [`crate::ErrorKind::of`] to read back, as
    /// [`with_kind`] tags an error
    ///
    /// The label and the tag are added in one `extend`, out of line (`label_with_kind`), so a
    /// derive-generated decoder's error path stays one call, which the inlining and register
    /// allocation of its success path depend on. The error itself is built inline: an error built
    /// out of line is returned through memory, and so is the decoded value sharing its `Result`
    ///
    /// Not part of the public API - may change without notice
    ///
    /// # Arguments
    ///
    /// * `input` - the stream the error is anchored to
    /// * `_checkpoint` - the position recorded before the failing step, which a
    ///   [`winnow::error::ContextError`] does not keep
    /// * `label` - a static description of what was being parsed
    /// * `kind` - what the error is
    ///
    /// # Returns
    ///
    /// A fresh error labelled `label` and tagged with `kind`
    #[must_use]
    #[inline]
    pub fn kinded_error<S>(
        input: &S,
        _checkpoint: &<S as winnow::stream::Stream>::Checkpoint,
        label: &'static str,
        kind: crate::ErrorKind,
    ) -> winnow::error::ContextError
    where
        S: winnow::stream::Stream,
    {
        use winnow::error::{ContextError, ParserError};
        let mut err = ContextError::from_input(input);
        label_with_kind(&mut err, label, kind);
        err
    }

    /// Labels `err` with `label` and tags it with `kind`, in one `extend`, for [`kinded_error`]
    ///
    /// # Arguments
    ///
    /// * `err` - the error to label, without a label of its own
    /// * `label` - a static description of what was being parsed
    /// * `kind` - what `err` is
    // as `with_kind`: `cold` keeps it out of the calling decoder's body, `inline` compiles it in
    // the calling crate
    #[cold]
    #[inline]
    fn label_with_kind(
        err: &mut winnow::error::ContextError,
        label: &'static str,
        kind: crate::ErrorKind,
    ) {
        use winnow::error::StrContext;
        err.extend([StrContext::Label(label), StrContext::Label(kind.label())]);
    }

    /// Names the site of a failed parse in `err`, so that its display shows `label` first, then
    /// why the parser failed
    ///
    /// [`winnow::error::ContextError`]'s display names only its first
    /// [`winnow::error::StrContext::Label`]. An `err` without a label gains `label` in place,
    /// keeping its context and cause. An `err` with one becomes the cause of a new error labelled
    /// `label`, then `err`'s first label, whose display prints `label`, then `err`'s own
    /// display. Neither path copies more than one label of `err`, so relabelling a value nested
    /// `n` deep costs `O(n)`. Either path then tags the error with `kind` ([`with_kind`])
    ///
    /// Not part of the public API - may change without notice
    ///
    /// # Arguments
    ///
    /// * `input` - the stream the error is anchored to
    /// * `checkpoint` - the position recorded before the failing step
    /// * `label` - a static description of what was being parsed
    /// * `err` - the error of the parser that failed there
    /// * `kind` - what the relabelled error is
    ///
    /// # Returns
    ///
    /// `err` with `label` added, when it carried no label of its own; otherwise a new error
    /// labelled `label`, then `err`'s first label, with `err` as its cause; either tagged with
    /// `kind`
    #[must_use]
    pub fn relabeled_error<S>(
        input: &S,
        checkpoint: &<S as winnow::stream::Stream>::Checkpoint,
        label: &'static str,
        err: winnow::error::ContextError,
        kind: crate::ErrorKind,
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
            return with_kind(
                err.add_context(input, checkpoint, StrContext::Label(label)),
                kind,
            );
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
        with_kind(relabeled, kind)
    }

    /// Tags `err` with `kind`, for [`crate::ErrorKind::of`] to read back
    ///
    /// The tag follows `err`'s labels, and [`winnow::error::ContextError`]'s display names only
    /// its first label, so the display is unchanged. `err` must carry a label already, or the
    /// tag would be the label its display names
    ///
    /// Not part of the public API - may change without notice
    ///
    /// # Arguments
    ///
    /// * `err` - the error to tag, labelled already
    /// * `kind` - what `err` is
    ///
    /// # Returns
    ///
    /// `err`, tagged with `kind`
    #[must_use]
    // `cold` keeps the tagging out of the calling decoder's body; `inline` compiles it in the
    // calling crate, which leaves that decoder's success path untouched in release
    #[cold]
    #[inline]
    pub fn with_kind(
        mut err: winnow::error::ContextError,
        kind: crate::ErrorKind,
    ) -> winnow::error::ContextError {
        use winnow::error::StrContext;
        debug_assert!(
            err.context()
                .any(|context| matches!(context, StrContext::Label(_))),
            "a kind tag ahead of every label would be the label the display names"
        );
        err.push(StrContext::Label(kind.label()));
        err
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
            typed_cause(&self.0)
        }
    }

    /// The cause of `err` that an error displaying `err` reports as its `source`: the first in
    /// its chain that is not a [`LabeledCause`], whose display `err`'s display already prints
    ///
    /// # Arguments
    ///
    /// * `err` - the error whose cause to find
    ///
    /// # Returns
    ///
    /// The cause, or `None` when the chain holds none but [`LabeledCause`]s
    pub(crate) fn typed_cause(
        err: &winnow::error::ContextError,
    ) -> Option<&(dyn std::error::Error + 'static)> {
        let mut cause = err.cause()?;
        while let Some(LabeledCause(inner)) = cause.downcast_ref::<LabeledCause>() {
            cause = inner.cause()?;
        }
        Some(cause)
    }

    /// Reads a value's text with `read`, then parses it with `parse`, for [`crate::as_date`],
    /// [`crate::as_time`] and [`crate::as_datetime`]
    ///
    /// Not part of the public API - may change without notice
    ///
    /// # Arguments
    ///
    /// * `input` - the stream the text is read from
    /// * `label` - what the value is, the label of either error
    /// * `read` - the decoder of the text, such as [`crate::dec::string::to_string_utf8`]
    /// * `parse` - the parser of the text
    ///
    /// # Returns
    ///
    /// What `parse` parsed
    ///
    /// # Errors
    ///
    /// - `read` fails: an error labelled `label`, of `read`'s kind, with `read`'s error as its
    ///   cause; a `read` error without a kind ([`crate::ErrorKind::of`]) gains `label` after its
    ///   own instead
    /// - `parse` fails: [`crate::ErrorKind::MalformedValue`], with `parse`'s error as its cause,
    ///   the cursor rewound to where the text started
    #[cfg(feature = "chrono")]
    pub fn parse_text<T, E>(
        input: &mut &[u8],
        label: &'static str,
        read: impl FnOnce(&mut &[u8]) -> crate::Result<String>,
        parse: impl FnOnce(&str) -> Result<T, E>,
    ) -> crate::Result<T>
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        use winnow::{
            error::{AddContext, ContextError, FromExternalError, StrContext},
            stream::Stream,
        };
        let start = input.checkpoint();
        let text = read(input).map_err(|err| match crate::ErrorKind::of(&err) {
            Some(kind) => relabeled_error(input, &start, label, err, kind),
            None => err.add_context(input, &start, StrContext::Label(label)),
        })?;
        parse(&text).map_err(|cause| {
            input.reset(&start);
            let err = ContextError::from_external_error(input, cause);
            relabeled_error(input, &start, label, err, crate::ErrorKind::MalformedValue)
        })
    }

    /// Calls a struct's `break_on` fn on the key just read and the input after it
    ///
    /// The bound on `f` is the signature a struct's `break_on` fn must have, so a fn of another
    /// signature is reported against it, with rustc's help to adjust the fn. `f` comes first, so
    /// `K` is the type the fn borrows and the key deref-coerces to it. It is taken by reference:
    /// for a fn passed by its bare path, rustc also suggests wrapping it in a closure, which
    /// `break_on` does not take
    ///
    /// Not part of the public API - may change without notice
    ///
    /// # Arguments
    ///
    /// * `f` - the struct's `break_on` fn
    /// * `key` - the key just read
    /// * `input` - the input right after the key
    ///
    /// # Returns
    ///
    /// What `f` decides
    #[inline(always)]
    pub fn break_on<K, S, F>(f: &F, key: &K, input: &mut S) -> crate::BreakType
    where
        K: ?Sized,
        F: Fn(&K, &mut S) -> crate::BreakType,
    {
        f(key, input)
    }

    /// Calls a field's `break_on` fn on the value just decoded
    ///
    /// The bound on `f` is the signature a field's `break_on` fn must have, so a fn of another
    /// signature is reported against it, with rustc's help to adjust the fn. `f` comes first, so
    /// `T` is the type the fn borrows and the value deref-coerces to it (a `String` to a `str`).
    /// It is taken by reference, as [`break_on`] takes its fn
    ///
    /// Not part of the public API - may change without notice
    ///
    /// # Arguments
    ///
    /// * `f` - the field's `break_on` fn
    /// * `value` - the value just decoded
    ///
    /// # Returns
    ///
    /// What `f` decides
    #[inline(always)]
    pub fn field_break_on<T, F>(f: &F, value: &T) -> crate::BreakType
    where
        T: ?Sized,
        F: Fn(&T) -> crate::BreakType,
    {
        f(value)
    }
}

/// Convenience re-export of all traits and parser primitives needed to work with KLV streams
///
/// Importing `tinyklv::prelude::*` brings into scope the [`Decoder`],
/// [`Packet`], the codec traits [`DecodeValue`], [`DecodePartial`],
/// [`EncodeValue`], [`EncodeAs`] and [`Partial`], the streaming traits
/// [`PartialIterator`] and [`ResumePartial`], the methods of
/// [`DecodeFrame`], [`DrainFrames`], [`EncodeFrame`] and [`SeekSentinel`]
/// (imported as `_`), the [`BreakType`] loop-control enum, the [`ErrorKind`]
/// of a decode error, the [`Framing`] fused-`keylen` item descriptor, the
/// [`Klv`] derive macro, and winnow's `Parser`, `ModalParser` and
/// `ModalResult`, with the methods of the error and stream traits that the
/// derive-generated code depends on
///
/// A crate that also glob-imports `std::io::*` and names `ErrorKind` gets
/// "`ErrorKind` is ambiguous", a warning or an error by compiler version;
/// importing the one it means by name, such as `use tinyklv::ErrorKind;`,
/// resolves it
pub mod prelude {
    // --------------------------------------------------
    // local
    // --------------------------------------------------
    pub use crate::{
        decoder::{Decoder, Packet},
        error::ErrorKind,
        framing::Framing,
        traits::{
            BreakType, DecodeFrame as _, DecodePartial, DecodeValue, DrainFrames as _, EncodeAs,
            EncodeFrame as _, EncodeValue, Partial, PartialIterator, ResumePartial,
            SeekSentinel as _,
        },
    };
    pub use tinyklv_impl::Klv;
    // --------------------------------------------------
    // external
    // --------------------------------------------------
    pub use winnow::{
        ModalParser, ModalResult, Parser,
        error::{AddContext as _, ModalError as _, ParserError as _},
        stream::{AsChar as _, ContainsToken as _, Stream as _, StreamIsPartial as _},
    };
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
/// Applies `encoder((*input / $scale) as $data)`, the inverse of [`scale!`] up
/// to the `as` cast, which rounds toward zero and saturates. The result is a
/// closure compatible with `#[klv(enc = ...)]`
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
/// Applies `encoder(*input as $data)`, the inverse of [`cast!`] up to the `as`
/// cast, which can round or saturate. The result is a closure compatible with
/// `#[klv(enc = ...)]`
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
            ::tinyklv::__export::parse_text(input, "date", $str_parser($len), |text| {
                ::tinyklv::__export::chrono::NaiveDate::parse_from_str(text, $date_fmt)
            })
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
            ::tinyklv::__export::parse_text(input, "time", $str_parser($len), |text| {
                ::tinyklv::__export::chrono::NaiveTime::parse_from_str(text, $time_fmt)
            })
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
            ::tinyklv::__export::parse_text(input, "datetime", $str_parser($len), |text| {
                ::tinyklv::__export::chrono::NaiveDateTime::parse_from_str(text, $datetime_fmt)
            })
        }
    };
}
