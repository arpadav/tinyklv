//! Break-condition types and trait for controlling the KLV field-decoding loop
//!
//! The derive-generated `decode_value` implementation loops over key-length-value
//! triples. After each key and length are decoded, [`BreakCondition::break_condition`]
//! is called to let the type decide what to do next. The return value is one
//! of four [`BreakConditionType`] variants that map directly to control-flow
//! actions inside the loop.
//!
//! Most types accept the default blanket implementation (always [`BreakConditionType::Proceed`]);
//! implement [`BreakCondition`] explicitly only when custom early-exit or skip
//! logic is needed.
//!
//! Author: aav
// --------------------------------------------------
// external
// --------------------------------------------------
use winnow::error::ContextError;

/// The four possible outcomes of a per-field break-condition check in the decode loop
///
/// Returned by [`BreakCondition::break_condition`] after each key and length
/// are parsed. The derive-generated loop branches on this value to decide
/// whether to decode the field, skip it, return early, or abort with an error.
#[non_exhaustive]
pub enum BreakConditionType {
    /// Do nothing in the decoding loop in [`crate::prelude::DecodeValue::decode_value`].
    ///
    /// This is the default, it just means there is nothing to be done and
    /// continue the decoding loop.
    ///
    /// Is named [`BreakConditionType::Proceed`] to refrain from using the keyword
    /// `continue`, since this does not use the reserved word `continue` and
    /// skip anything in the loop.
    ///
    /// This is equivalent to:
    ///
    /// ```rust ignore
    /// loop {
    ///     match Self::break_condition(key, len) {
    ///         BreakConditionType::Proceed => (),
    ///         _ => ..., // see `BreakConditionType`
    ///     }
    ///     // decoding logic
    /// }
    /// // return
    /// ```
    Proceed,

    /// Skips the current value in [`crate::prelude::DecodeValue::decode_value`]
    ///
    /// This is useful for skipping fields that aren't implemented yet,
    /// by skipping over them and continuing the decoding loop.
    ///
    /// This is equivalent to:
    ///
    /// ```rust ignore
    /// loop {
    ///     match Self::break_condition(key, len) {
    ///         BreakConditionType::Skip => {
    ///             take(len).parse_next(input); // error handled properly in macro expansion
    ///             continue;
    ///         },
    ///         _ => ..., // see `BreakConditionType`
    ///     }
    ///     // decoding logic
    /// }
    /// // return
    /// ```
    Skip,

    /// Returns the decoded value, if all required fields are present.
    ///
    /// This does not guarantee to return [`Ok`] from
    /// [`crate::prelude::DecodeValue::decode_value`], since required fields might not
    /// be present.
    ///
    /// This is useful if some un-recoverable issue has occurred but we
    /// still want to return a partially-parsed result.
    ///
    /// For example, if `len` is decoded to be greater than the maximum
    /// size of the packet, then clearly the packet is malformed and we
    /// should at least try to return what has already been parsed.
    ///
    /// This is equivalent to:
    ///
    /// ```rust ignore
    /// loop {
    ///     match Self::break_condition(key, len) {
    ///         BreakConditionType::Done => break,
    ///         _ => ..., // see `BreakConditionType`
    ///     }
    ///     // decoding logic
    /// }
    /// return Ok(/* check if required fields are present */);
    /// ```
    Done,

    /// Returns an error from [`crate::prelude::DecodeValue::decode_value`]
    /// without returning any potential decoded values.
    ///
    /// This is should only be used in cases of a fatal error, since an
    /// [`Err`] is **guaranteed** to return from [`crate::prelude::DecodeValue::decode_value`].
    ///
    /// This is equivalent to:
    ///
    /// ```rust ignore
    /// loop {
    ///     match Self::break_condition(key, len) {
    ///         BreakConditionType::Abort(e) => return Err(e),
    ///         _ => ..., // see `BreakConditionType`
    ///     }
    ///     // decoding logic
    /// }
    /// // return
    /// ```
    Abort(ContextError),
}

/// Allows a type to inspect each decoded key and length and control the decode loop
///
/// The derive macro calls [`BreakCondition::break_condition`] inside the
/// field-dispatch loop after every key and length decode. The default
/// implementation always returns [`BreakConditionType::Proceed`], which is
/// correct for the vast majority of types.
///
/// Override this method to:
/// * skip unrecognised tags ([`BreakConditionType::Skip`])
/// * stop early when a sentinel tag is seen ([`BreakConditionType::Done`])
/// * abort with a hard error on an impossible length ([`BreakConditionType::Abort`])
///
/// # Arguments
///
/// * `decoded_key` - The tag/key value just decoded from the stream
/// * `decoded_len` - The length value just decoded from the stream
///
/// # Returns
///
/// A [`BreakConditionType`] directing the decode loop how to proceed
pub trait BreakCondition<S> {
    #[inline(always)]
    #[allow(unused_variables)]
    fn break_condition<K, L>(decoded_key: K, decoded_len: L) -> BreakConditionType {
        BreakConditionType::Proceed
    }
}
/// [`BreakCondition`] blanket implementation for all types `T` that implement [`DecodeValue`]
///
/// Provides the default "always proceed" behaviour for every type that can
/// be decoded. Override [`BreakCondition::break_condition`] on the concrete
/// type when custom loop-control logic is required.
impl<T, S> BreakCondition<S> for T
where
    T: crate::traits::dec::DecodeValue<S>,
    S: winnow::stream::Stream,
{
}
