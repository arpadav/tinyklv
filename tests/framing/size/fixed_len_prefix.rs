//! Fixed-width length prefix bound tests
//!
//! A one-byte `len(size(exact = 1))` prefix encodes lengths up to 255: a field declared
//! `size(exact = 255)` compiles and round-trips, while `size(exact = 256)` is a compile error
//! (pinned in `tests/ui`). A field with no `size(exact)` reserves the slot and back-fills it:
//! a 255-byte value is written, a 256-byte one rolls its item back. A two-byte and an eight-byte
//! slot back-fill their lengths byte for byte. A length encoder that writes more bytes than its
//! declared `size` writes a known-width field's length as it is, while a value encoder that
//! writes another width than its field's `size(exact)` fails the debug check
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
use winnow::{
    Parser,
    token::{rest, take},
};

// --------------------------------------------------
// constants
// --------------------------------------------------
/// The widest value a one-byte length prefix encodes
const MAX_ONE_BYTE_LEN: usize = 255;

/// A value length whose two low bytes both are non-zero, so a back-filled slot shows their order
const TWO_BYTE_LEN: usize = 0x0102;

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

/// An opaque record of any byte length, so the field holding it declares no `size(exact)`
#[derive(Debug, PartialEq)]
struct Certificate(Vec<u8>);

/// [`Certificate`] implementation
impl Certificate {
    /// A certificate of `len` bytes counting up from zero
    fn of_len(len: usize) -> Self {
        Self((0..=u8::MAX).cycle().take(len).collect())
    }

    /// Decodes every byte of the value
    fn decode(input: &mut &[u8]) -> tinyklv::Result<Self> {
        rest.map(|bytes: &[u8]| Self(bytes.to_vec()))
            .parse_next(input)
    }

    /// Encodes the record's bytes
    fn encode(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.0);
    }
}

/// A one-byte length prefix in front of a value of unknown width: the encoder reserves the slot,
/// writes the value, then back-fills the slot or rolls the item back
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize, size(exact = 1)),
)]
struct Endorsement {
    #[klv(key = 0x01, dec = decb::u8, enc = *encb::u8)]
    serial: u8,

    #[klv(key = 0x02, dec = Certificate::decode, enc = Certificate::encode)]
    certificate: Option<Certificate>,
}

/// A two-byte length prefix in front of a value of unknown width
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::be_u16_as_usize, enc = encb::be_u16_from_usize, size(exact = 2)),
)]
struct Bundle {
    #[klv(key = 0x01, dec = Certificate::decode, enc = Certificate::encode)]
    certificate: Certificate,
}

/// An eight-byte length prefix, as wide as a 64-bit `usize`, in front of a value of unknown width
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::be_u64_as_usize, enc = encb::be_u64_from_usize, size(exact = 8)),
)]
struct Archive {
    #[klv(key = 0x01, dec = Certificate::decode, enc = Certificate::encode)]
    certificate: Certificate,
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

/// Writes a setpoint's low byte alone, one of the two bytes its field declares
fn low_byte(setpoint: u16, out: &mut Vec<u8>) {
    out.push(setpoint as u8);
}

/// A value encoder narrower than its field's `size(exact = 2)`, under a one-byte length slot
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize, size(exact = 1)),
)]
struct Dial {
    #[klv(
        key = 0x01,
        dec = decb::be_u16,
        enc = *low_byte,
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
/// A 255-byte value of unknown width fills its back-filled one-byte length slot exactly
fn widest_backfilled_value_is_written() {
    let certificate = Certificate::of_len(MAX_ONE_BYTE_LEN);
    #[rustfmt::skip]
    let mut wire = vec![
        // serial = 7
        0x01, 0x01, 0x07,
        // certificate, its length 255 back-filled
        0x02, 0xFF,
    ];
    wire.extend_from_slice(&certificate.0);
    let endorsement = Endorsement {
        serial: 7,
        certificate: Some(certificate),
    };
    let mut encoded = Vec::new();
    endorsement.encode_value(&mut encoded);
    assert_eq!(encoded, wire);
    assert_eq!(
        Endorsement::decode_value(&mut encoded.as_slice()),
        Ok(endorsement)
    );
}

#[test]
/// A 256-byte value of unknown width overflows its one-byte length slot: the encoder rolls the
/// whole item back and leaves the serial before it untouched
fn backfilled_value_past_the_slot_is_omitted() {
    let endorsement = Endorsement {
        serial: 7,
        certificate: Some(Certificate::of_len(MAX_ONE_BYTE_LEN + 1)),
    };
    let mut encoded = Vec::new();
    endorsement.encode_value(&mut encoded);
    // --------------------------------------------------
    // serial = 7, and nothing of the certificate
    // --------------------------------------------------
    assert_eq!(encoded, [0x01, 0x01, 0x07]);
    assert_eq!(
        Endorsement::decode_value(&mut encoded.as_slice()),
        Ok(Endorsement {
            serial: 7,
            certificate: None,
        })
    );
}

#[test]
/// A [`TWO_BYTE_LEN`]-byte value of unknown width back-fills its two-byte length slot, high byte
/// first
fn backfilled_two_byte_len_is_written() {
    let certificate = Certificate::of_len(TWO_BYTE_LEN);
    #[rustfmt::skip]
    let mut wire = vec![
        // certificate, its length 0x0102 back-filled
        0x01, 0x01, 0x02,
    ];
    wire.extend_from_slice(&certificate.0);
    let bundle = Bundle { certificate };
    let mut encoded = Vec::new();
    bundle.encode_value(&mut encoded);
    assert_eq!(encoded, wire);
    assert_eq!(Bundle::decode_value(&mut encoded.as_slice()), Ok(bundle));
}

#[test]
/// A [`TWO_BYTE_LEN`]-byte value of unknown width back-fills its eight-byte length slot, whose
/// largest length saturates at `usize::MAX`, high byte first
fn backfilled_eight_byte_len_is_written() {
    let certificate = Certificate::of_len(TWO_BYTE_LEN);
    #[rustfmt::skip]
    let mut wire = vec![
        // certificate, its length 0x0102 back-filled
        0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x02,
    ];
    wire.extend_from_slice(&certificate.0);
    let archive = Archive { certificate };
    let mut encoded = Vec::new();
    archive.encode_value(&mut encoded);
    assert_eq!(encoded, wire);
    assert_eq!(Archive::decode_value(&mut encoded.as_slice()), Ok(archive));
}

#[test]
/// A length encoder that overruns its declared slot writes a known-width field's length as it is
fn wide_len_encoder_writes_a_known_width_length_as_is() {
    #[rustfmt::skip]
    let wire = [
        // key, then the length 2 as `two_byte_len` writes it
        0x01, 0x00, 0x02,
        // setpoint = 0x0102
        0x01, 0x02,
    ];
    let mut encoded = Vec::new();
    Thermostat { setpoint: 0x0102 }.encode_value(&mut encoded);
    assert_eq!(encoded, wire);
}

#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "wrote a different byte count than its fixed width")]
/// A value encoder that writes another width than its field's `size(exact = 2)` fails the debug
/// check: the slot holds the declared width, never a length back-filled from the bytes written
fn narrow_value_encoder_fails_debug_check() {
    let mut encoded = Vec::new();
    Dial { setpoint: 0x0102 }.encode_value(&mut encoded);
}
