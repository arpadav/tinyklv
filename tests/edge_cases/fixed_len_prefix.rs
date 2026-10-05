//! Fixed-width length prefix bound tests
//!
//! A one-byte `len(size(exact = 1))` prefix encodes lengths up to 255: a field declared
//! `size(exact = 255)` compiles and round-trips, while `size(exact = 256)` is a compile error
//! (pinned in `tests/ui`). A length encoder that writes more bytes than its declared `size` fails
//! the debug check on a known-width field
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use tinyklv::dec::binary as decb;
use tinyklv::enc::binary as encb;
use tinyklv::prelude::*;

// --------------------------------------------------
// external
// --------------------------------------------------
use winnow::{Parser, token::take};

// --------------------------------------------------
// constants
// --------------------------------------------------
/// The widest value a one-byte length prefix encodes
const MAX_ONE_BYTE_LEN: usize = 255;

/// An opaque record that encodes to exactly [`MAX_ONE_BYTE_LEN`] bytes
#[derive(Debug, PartialEq)]
struct Fingerprint([u8; MAX_ONE_BYTE_LEN]);

/// [`Fingerprint`] implementation
impl Fingerprint {
    /// Decodes the record's [`MAX_ONE_BYTE_LEN`] bytes
    fn decode(input: &mut &[u8]) -> tinyklv::Result<Self> {
        let mut bytes = [0u8; MAX_ONE_BYTE_LEN];
        bytes.copy_from_slice(take(MAX_ONE_BYTE_LEN).parse_next(input)?);
        Ok(Self(bytes))
    }

    /// Encodes the record's [`MAX_ONE_BYTE_LEN`] bytes
    fn encode(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.0);
    }
}

/// A one-byte length prefix carrying the widest value it can encode
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize, size(exact = 1)),
)]
struct Attestation {
    #[klv(
        key = 0x01,
        dec = Fingerprint::decode,
        enc = Fingerprint::encode,
        size(exact = 255),
    )]
    fingerprint: Fingerprint,
}

/// Writes a length as two big-endian bytes, though the container declares a one-byte slot
fn two_byte_len(len: usize, out: &mut Vec<u8>) {
    out.push(0);
    encb::u8_from_usize(len, out);
}

/// A length encoder wider than its declared `size(exact = 1)`, on a known-width field
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = two_byte_len, size(exact = 1)),
)]
struct Thermostat {
    #[klv(
        key = 0x01,
        dec = decb::be_u16,
        enc = *encb::be_u16,
        size(exact = 2),
    )]
    setpoint: u16,
}

#[test]
/// A 255-byte value fills a one-byte length prefix exactly and round-trips
fn widest_value_fits_one_byte_len() {
    let mut bytes = [0u8; MAX_ONE_BYTE_LEN];
    bytes.iter_mut().zip(0u8..).for_each(|(byte, n)| *byte = n);
    let packet = Attestation {
        fingerprint: Fingerprint(bytes),
    };
    let mut encoded = Vec::new();
    packet.encode_value(&mut encoded);
    assert_eq!(encoded[..2], [0x01, 0xFF], "key, then the length 255");
    assert_eq!(encoded.len(), 2 + MAX_ONE_BYTE_LEN);
    assert_eq!(encoded[2..], packet.fingerprint.0);
    let decoded = Attestation::decode_value(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, packet);
}

#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "length encoder wrote a different byte count")]
/// A length encoder that overruns its declared slot fails the debug check on a known-width field
fn wide_len_encoder_fails_debug_check() {
    let mut encoded = Vec::new();
    Thermostat { setpoint: 0x0102 }.encode_value(&mut encoded);
}
