//! The [`BreakType`] outcomes that control the KLV field-decoding loop
//!
//! The derive-generated decode loop (`decode_value` and the streaming `resume_partial`) reads
//! key-length-value triples. When a container declares `#[klv(break_on = ..)]`, the macro evaluates
//! the break expression after each key and length are decoded and branches on the resulting
//! [`BreakType`] to decide what the loop does next. Without `break_on`, the loop always proceeds.
//!
//! `break_on` accepts either a key literal - a decoded key equal to the literal yields
//! [`BreakType::Done`] - or a function `fn(key, len) -> BreakType` returning any variant.
//!
//! Author: aav

/// The four possible outcomes of a per-field break check in the decode loop
///
/// Produced by a `#[klv(break_on = ..)]` expression after each key and length are parsed. The
/// derive-generated loop branches on this value to decide whether to decode the field, skip it,
/// return early, or abort with an error.
#[non_exhaustive]
pub enum BreakType {
    /// Continue the decoding loop normally - decode the current field, then read the next triple.
    ///
    /// This is the default outcome (and the only one when no `break_on` is declared). Named
    /// `Proceed` rather than reusing the `continue` keyword's sense: it does NOT skip the current
    /// field's decode, it simply lets the loop carry on.
    ///
    /// Equivalent to:
    ///
    /// ```rust ignore
    /// loop {
    ///     match /* break_on(key, len) */ {
    ///         BreakType::Proceed => (),
    ///         _ => ..., // see `BreakType`
    ///     }
    ///     // decoding logic
    /// }
    /// // return
    /// ```
    Proceed,

    /// Consume the current value and continue, without decoding it into a field.
    ///
    /// Useful for skipping reserved or not-yet-implemented tags: the value bytes are taken
    /// (advancing past them) and the loop continues to the next triple.
    ///
    /// Equivalent to:
    ///
    /// ```rust ignore
    /// loop {
    ///     match /* break_on(key, len) */ {
    ///         BreakType::Skip => {
    ///             take(len).parse_next(input); // error handled in the macro expansion
    ///             continue;
    ///         },
    ///         _ => ..., // see `BreakType`
    ///     }
    ///     // decoding logic
    /// }
    /// // return
    /// ```
    Skip,

    /// Stop the loop and return the decoded value, if all required fields are present.
    ///
    /// This does not guarantee an [`Ok`] from [`crate::prelude::DecodeValue::decode_value`], since
    /// required fields might still be missing - it stops reading further triples and finalizes
    /// whatever has been accumulated. This is the outcome a `break_on = <literal>` produces when a
    /// terminator key is matched.
    ///
    /// Equivalent to:
    ///
    /// ```rust ignore
    /// loop {
    ///     match /* break_on(key, len) */ {
    ///         BreakType::Done => break,
    ///         _ => ..., // see `BreakType`
    ///     }
    ///     // decoding logic
    /// }
    /// return Ok(/* check if required fields are present */);
    /// ```
    Done,

    /// Return an error immediately from [`crate::prelude::DecodeValue::decode_value`], surfacing no
    /// partially-decoded value.
    ///
    /// Use only for fatal conditions (e.g. an impossible length): an [`Err`] carrying the supplied
    /// static message is **guaranteed** to return. The message flows through both the one-shot and
    /// streaming decode paths.
    ///
    /// Equivalent to:
    ///
    /// ```rust ignore
    /// loop {
    ///     match /* break_on(key, len) */ {
    ///         BreakType::Abort(msg) => return Err(/* error labelled with msg */),
    ///         _ => ..., // see `BreakType`
    ///     }
    ///     // decoding logic
    /// }
    /// // return
    /// ```
    Abort(&'static str),
}
