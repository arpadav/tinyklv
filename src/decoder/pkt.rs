//! The [`Packet`] enum: the two-state signal produced by a single partial-decode attempt
//!
//! A [`Packet`] distinguishes between a successfully completed decode
//! ([`Packet::Ready`]) and a decode that consumed all available bytes without
//! finishing ([`Packet::NeedMore`]). Malformed input is not a [`Packet`] state;
//! it surfaces as a [`crate::traits::PartialError`] on the outer `Result` returned by
//! [`crate::traits::DecodePartial::decode_partial`]
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::traits::Partial;

#[derive(Debug)]
/// Result of a single [`crate::traits::DecodePartial::decode_partial`] attempt
///
/// Two states only:
///
/// * [`Packet::Ready`] - the partial finalised cleanly into `T`. Input
///   cursor is advanced past the consumed bytes
/// * [`Packet::NeedMore`] - the decode loop paused because more input
///   is needed. The partial holds every KLV field that landed so far.
///   Call [`crate::traits::ResumePartial::resume_partial`] with it once
///   more bytes are available. Malformed input surfaces as a
///   [`crate::traits::PartialError`] on the outer `Result` from
///   [`crate::traits::DecodePartial::decode_partial`]
pub enum Packet<T, P>
where
    P: Partial<Final = T>,
{
    /// Decode succeeded; `T` is the value and `input` has advanced past it
    Ready(T),

    /// Decode needs more bytes. The partial holds every klv field
    /// landed so far. Hand it back to [`crate::traits::ResumePartial::resume_partial`]
    /// once more bytes are available
    NeedMore(P),
}
