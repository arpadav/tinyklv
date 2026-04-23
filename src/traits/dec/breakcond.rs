// --------------------------------------------------
// external
// --------------------------------------------------
use winnow::error::ContextError;

/// Decoding-loop break types
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

/// A trait for breaking during during decoding loop
pub trait BreakCondition<S> {
    #[inline(always)]
    #[allow(unused_variables)]
    fn break_condition<K, L>(decoded_key: K, decoded_len: L) -> BreakConditionType {
        BreakConditionType::Proceed
    }
}
/// [`BreakCondition`] blanket implementation for all types `T` that implement [`DecodeValue`]
impl<T, S> BreakCondition<S> for T
where
    T: crate::traits::dec::DecodeValue<S>,
    S: winnow::stream::Stream,
{
}
