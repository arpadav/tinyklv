//! `size(exact = N)` on a struct decoded from a stream whose state is not `Clone`
//!
//! One-shot decode of such a struct advances the caller's cursor, so the stream need not be
//! `Clone`, and leaves it where a `&[u8]` stream's is left: past a rejected frame the input holds
//! whole, at the start of one that runs past the end of the input
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::common::error::ContextErrorExt;
use tinyklv::dec::binary as decb;
use tinyklv::prelude::*;

// --------------------------------------------------
// external
// --------------------------------------------------
use winnow::stream::Stateful;

/// The frame headers a decode has read; a decode cannot copy it
#[derive(Debug)]
struct Tally(usize);

/// A byte stream that counts the frame headers read from it
type Counted = Stateful<&'static [u8], Tally>;

/// Reads a one-byte key, counting the frame header
fn dec_key(input: &mut Counted) -> winnow::Result<u8> {
    input.state.0 += 1;
    winnow::binary::u8.parse_next(input)
}

/// Reads a one-byte length
fn dec_len(input: &mut Counted) -> winnow::Result<usize> {
    winnow::binary::u8.map(usize::from).parse_next(input)
}

/// A water level read off a counted stream
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = Counted,
    key(dec = dec_key),
    len(dec = dec_len),
    allow_unimplemented_encode,
)]
struct Gauge {
    /// a big-endian `u16`, exactly 2 bytes
    #[klv(key = 0x01, dec = decb::be_u16, size(exact = 2))]
    level: u16,
}

/// One-shot decodes `wire` from a fresh tally
///
/// # Returns
///
/// The decoded value or the error's labels, the offset the cursor stopped at, and the frame
/// headers read
fn decode(wire: &'static [u8]) -> (Result<Gauge, Vec<&'static str>>, usize, usize) {
    let mut input = Counted {
        input: wire,
        state: Tally(0),
    };
    let outcome = Gauge::decode_value(&mut input).map_err(|error| error.labels().collect());
    (outcome, wire.len() - input.input.len(), input.state.0)
}

/// A frame of the exact width decodes; one of another width is rejected past it when the input
/// holds it whole, and at its start when it runs past the end of the input
#[test]
fn an_exact_field_decodes_from_a_stream_that_is_not_clone() {
    let message = "`Gauge::level` value (its frame length does not match `size(exact = 2)`)";
    // level = 42
    let decoded = decode(&[0x01, 0x02, 0x00, 0x2A]);
    assert_eq!(decoded, (Ok(Gauge { level: 42 }), 4, 1));
    // level, 3 bytes
    let (outcome, stopped, _) = decode(&[0x01, 0x03, 0x00, 0x00, 0x2A]);
    assert_eq!((outcome, stopped), (Err(vec![message]), 5));
    // level, a 64-byte length with 2 bytes after it
    let (outcome, stopped, _) = decode(&[0x01, 0x40, 0x00, 0x2A]);
    assert_eq!((outcome, stopped), (Err(vec![message]), 0));
}
