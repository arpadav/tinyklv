//! Cross-approach equivalence gate
//!
//! Before any timing, [`verify`] asserts that every approach round-trips the canonical
//! sample identically across all three decode tiers - `value`, `frame`, and `streamed`. A
//! divergence aborts the bench loudly, so the numbers can never compare approaches that decode
//! different things. The `streamed` check also doubles as a correctness gate on the multi-packet
//! decode path (tinyklv's streaming `Decoder`, the others' seek loop)
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::suite::approaches::{
    Manual, Micropb, Prost, QuickProtobuf, RustProtobuf, SerdeKlv, Tinyklv, TlvParser,
};
use crate::suite::{Codec, framing};

/// Asserts every approach round-trips `sample` identically across the value, frame, and streamed
/// tiers
///
/// # Arguments
///
/// * `sample` - the canonical record each approach must reproduce
///
/// # Panics
///
/// Panics if any approach's value, frame, or streamed round-trip diverges from `sample`
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

/// Asserts one approach reproduces `sample` across all three decode tiers
///
/// 1. **value** - `A::decode(A::encode(sample))` equals `sample`
/// 2. **frame** - `A::decode_framed(A::encode_framed(sample))` equals `sample` (clean frame, the
///    exact input the `frame` tier times - no stream noise)
/// 3. **streamed** - `A::decode_streamed` over a [`framing::make_stream`] buffer yields exactly
///    [`framing::STREAM_PACKETS`] copies of `sample`, validating the multi-packet decode path
///
/// # Arguments
///
/// * `sample` - the canonical record the approach must round-trip exactly
///
/// # Panics
///
/// Panics if any of the three tiers produces a result that diverges from `sample`, or if
/// `decode_streamed` returns `None` or yields the wrong packet count. The panic message names
/// the failing approach and which tier diverged
fn verify_one<A, R>(sample: &R)
where
    A: Codec<R>,
    R: PartialEq + core::fmt::Debug,
{
    // --------------------------------------------------
    // value: bare encode/decode round-trip
    // --------------------------------------------------
    assert_eq!(
        A::decode(&A::encode(sample)).as_ref(),
        Some(sample),
        "{}: value round-trip diverged",
        A::NAME,
    );
    // --------------------------------------------------
    // frame: single clean frame round-trip (no noise)
    // --------------------------------------------------
    assert_eq!(
        A::decode_framed(&A::encode_framed(sample)).as_ref(),
        Some(sample),
        "{}: frame round-trip diverged",
        A::NAME,
    );
    // --------------------------------------------------
    // streamed: every packet in a multi-packet noisy buffer decodes back to `sample`
    // --------------------------------------------------
    let stream = framing::make_stream(&A::encode_framed(sample));
    let decoded = A::decode_streamed(&stream)
        .unwrap_or_else(|| panic!("{}: streamed decode failed", A::NAME));
    assert_eq!(
        decoded.len(),
        framing::STREAM_PACKETS,
        "{}: streamed packet count diverged",
        A::NAME,
    );
    for (i, got) in decoded.iter().enumerate() {
        assert_eq!(got, sample, "{}: streamed packet {i} diverged", A::NAME);
    }
}
