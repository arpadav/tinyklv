//! Cross-approach equivalence gate.
//!
//! Before any timing, [`verify`] asserts that every approach round-trips the canonical
//! sample identically - both clean and framed. A divergence aborts the bench loudly, so
//! the numbers can never compare approaches that decode different things.
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::suite::approaches::{
    Manual, Micropb, Prost, QuickProtobuf, RustProtobuf, SerdeKlv, Tinyklv, TlvParser,
};
use crate::suite::{framing, Codec};

/// Asserts every approach round-trips `sample` identically, both clean and framed.
///
/// # Arguments
///
/// * `sample` - the canonical record each approach must reproduce.
///
/// # Panics
///
/// Panics if any approach's clean or framed round-trip diverges from `sample`.
pub(crate) fn verify<R>(sample: &R)
where
    R: PartialEq + core::fmt::Debug,
    Tinyklv: Codec<R>,
    SerdeKlv: Codec<R>,
    TlvParser: Codec<R>,
    Manual: Codec<R>,
    Prost: Codec<R>,
    QuickProtobuf: Codec<R>,
    RustProtobuf: Codec<R>,
    Micropb: Codec<R>,
{
    verify_one::<Tinyklv, R>(sample);
    verify_one::<SerdeKlv, R>(sample);
    verify_one::<TlvParser, R>(sample);
    verify_one::<Manual, R>(sample);
    verify_one::<Prost, R>(sample);
    verify_one::<QuickProtobuf, R>(sample);
    verify_one::<RustProtobuf, R>(sample);
    verify_one::<Micropb, R>(sample);
}

/// Asserts one approach's clean and framed round-trips both reproduce `sample`
///
/// Encodes `sample` with `A::encode`, decodes it with `A::decode`, and asserts
/// equality. Then wraps the frame in stream noise with [`framing::make_noisy`]
/// and asserts `A::decode_framed` also reproduces `sample`. Panics on any
/// divergence with a message naming the approach
///
/// # Arguments
///
/// * `sample` - the canonical record the approach must round-trip exactly
fn verify_one<A, R>(sample: &R)
where
    A: Codec<R>,
    R: PartialEq + core::fmt::Debug,
{
    // --------------------------------------------------
    // assert clean encode/decode round-trip
    // --------------------------------------------------
    assert_eq!(
        A::decode(&A::encode(sample)).as_ref(),
        Some(sample),
        "{}: clean round-trip diverged",
        A::NAME,
    );
    // --------------------------------------------------
    // assert framed encode/decode round-trip through stream noise
    // --------------------------------------------------
    let noisy = framing::make_noisy(&A::encode_framed(sample));
    assert_eq!(
        A::decode_framed(&noisy).as_ref(),
        Some(sample),
        "{}: framed round-trip diverged",
        A::NAME,
    );
}
