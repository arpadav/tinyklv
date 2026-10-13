//! The [`BreakType`] outcomes that control the KLV field-decoding loop
//!
//! The derive-generated decode loop (`decode_value` and the streaming `resume_partial`) reads
//! key-length-value triples. When a container declares `#[klv(break_on = ..)]`, the macro evaluates
//! the break expression right after each key, before the loop reads anything else (in `keylen`
//! mode, right after the header), and branches on the resulting [`BreakType`] to decide what the
//! loop does next. Without `break_on`, the loop always proceeds
//!
//! `break_on` accepts either a key literal - a decoded key equal to the literal yields
//! [`BreakType::Done`] - or a function `fn(&K, &mut S) -> BreakType` handed the key and the input
//! right after it. `Proceed` rewinds the input to just after the key, so the fn may read on to
//! decide; every other variant leaves the input where the fn did. A fn that reads past the key
//! returns `Proceed` on a short read, so the frame reads as cut and streaming waits for more
//!
//! A field's `#[klv(break_on = ..)]` is a function `fn(&value) -> BreakType`, evaluated after that
//! field's value decodes: `Proceed` keeps the value, `Skip` discards it, `Done` keeps it and ends
//! the decode, and `Abort` is an error. Encode writes the fields with a `break_on` last
//!
//! Author: aav

/// The four possible outcomes of a `break_on` check in the decode loop
///
/// Produced by a container's `#[klv(break_on = ..)]` expression right after each key, and by a
/// field's after its value decodes. The derive-generated loop branches on this value to decide
/// whether to decode the field, skip it, return early, or abort with an error. The variants below
/// describe the container's check; the module docs give a field's
#[non_exhaustive]
pub enum BreakType {
    /// Continue the decoding loop normally - decode the current field, then read the next triple
    ///
    /// This is the default outcome (and the only one when no `break_on` is declared). Named
    /// `Proceed` rather than reusing the `continue` keyword's sense: it does NOT skip the current
    /// field's decode, it simply lets the loop carry on from just after the key
    ///
    /// Equivalent to:
    ///
    /// ```rust ignore
    /// loop {
    ///     let after_key = input.checkpoint();
    ///     match /* break_on(&key, input) */ {
    ///         BreakType::Proceed => input.reset(&after_key),
    ///         _ => ..., // see `BreakType`
    ///     }
    ///     // decoding logic
    /// }
    /// // return
    /// ```
    Proceed,

    /// Continue from where the fn left the input, without decoding the frame into a field
    ///
    /// Useful for skipping reserved or not-yet-implemented tags: the fn reads past the frame's
    /// length and value itself, and the loop reads the next triple from there
    ///
    /// Equivalent to:
    ///
    /// ```rust ignore
    /// loop {
    ///     match /* break_on(&key, input) */ {
    ///         BreakType::Skip => continue,
    ///         _ => ..., // see `BreakType`
    ///     }
    ///     // decoding logic
    /// }
    /// // return
    /// ```
    Skip,

    /// Stop the loop and return the decoded value, if all required fields are present
    ///
    /// This does not guarantee an [`Ok`] from [`crate::prelude::DecodeValue::decode_value`], since
    /// required fields might still be missing - it stops reading further triples and finalizes
    /// whatever has been accumulated. This is the outcome a `break_on = <literal>` produces when a
    /// terminator key is matched
    ///
    /// Equivalent to:
    ///
    /// ```rust ignore
    /// loop {
    ///     match /* break_on(&key, input) */ {
    ///         BreakType::Done => break,
    ///         _ => ..., // see `BreakType`
    ///     }
    ///     // decoding logic
    /// }
    /// return Ok(/* check if required fields are present */);
    /// ```
    Done,

    /// Return an error immediately from [`crate::prelude::DecodeValue::decode_value`], surfacing no
    /// partially-decoded value
    ///
    /// Use only for fatal conditions (e.g. an impossible length): an [`Err`] carrying the supplied
    /// static message is **guaranteed** to return. The message flows through both the one-shot and
    /// streaming decode paths
    ///
    /// Equivalent to:
    ///
    /// ```rust ignore
    /// loop {
    ///     match /* break_on(&key, input) */ {
    ///         BreakType::Abort(msg) => return Err(/* error labelled with msg */),
    ///         _ => ..., // see `BreakType`
    ///     }
    ///     // decoding logic
    /// }
    /// // return
    /// ```
    Abort(&'static str),
}
