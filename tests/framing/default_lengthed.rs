//! A container `default(typ = T, ..)`'s `lengthed` goes with its decoder
//!
//! A field of `T` that brings its own decoder is `lengthed` only if it says so, so a decoder that
//! takes no length works under a `lengthed` default, and still takes the default's width. A field
//! on the default's decoder takes the default's `lengthed`, whatever width of its own it sets, and
//! may write `lengthed` itself
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::common::drive::Drive;
use crate::common::varint;
use tinyklv::dec::string as decs;
use tinyklv::prelude::*;

// --------------------------------------------------
// external
// --------------------------------------------------
use winnow::token::rest;

/// A radio operator's callsign
#[derive(Debug, Clone, PartialEq)]
struct Callsign(String);

/// [`Callsign`] implementation
impl Callsign {
    /// Decodes a callsign of `width` bytes
    ///
    /// # Arguments
    ///
    /// * `width` - the bytes to read, the length the derive hands a `lengthed` decoder
    ///
    /// # Returns
    ///
    /// A decoder reading `width` bytes, or failing on fewer
    fn decode_lengthed(width: usize) -> impl Fn(&mut &[u8]) -> tinyklv::Result<Self> {
        move |input| decs::to_string_utf8(width).map(Self).parse_next(input)
    }

    /// Decodes a callsign from every byte of the input, handed no length
    ///
    /// # Arguments
    ///
    /// * `input` - the frame's value bytes; fully consumed
    ///
    /// # Returns
    ///
    /// The callsign
    fn decode_rest(input: &mut &[u8]) -> tinyklv::Result<Self> {
        rest.map(|bytes: &[u8]| Self(String::from_utf8_lossy(bytes).into_owned()))
            .parse_next(input)
    }
}

/// A radio log entry whose callsigns use the `Callsign` default: a `lengthed` decoder, six bytes
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = varint::dec_u32, enc = varint::enc_u32),
    len(dec = varint::dec_usize, enc = varint::enc_usize),
    allow_unimplemented_encode,
    default(
        typ = Callsign,
        dec = Callsign::decode_lengthed,
        size(lengthed, exact = 6),
    ),
)]
struct RadioLog {
    /// field 1 - everything from the default: `lengthed`, six bytes
    #[klv(key = 0x0A)]
    caller: Callsign,

    /// field 2 - its own `dec` takes no length, so it is not `lengthed`; six bytes from the
    /// default
    #[klv(key = 0x12, dec = Callsign::decode_rest)]
    callee: Callsign,

    /// field 3 - optional, its own `size(exact = 4)`; the default's decoder, so `lengthed`
    #[klv(key = 0x1A, size(exact = 4))]
    relay: Option<Callsign>,

    /// field 4 - optional, its own `size(lengthed)`, as the default's decoder is; six bytes from
    /// the default
    #[klv(key = 0x22, size(lengthed))]
    witness: Option<Callsign>,
}

/// [`RadioLog`] implementation
impl RadioLog {
    /// caller = "N0CALL" (field 1, 6 bytes)
    const CALLER: &[u8] = &[0x0A, 0x06, b'N', b'0', b'C', b'A', b'L', b'L'];

    /// callee = "DL1ABC" (field 2, 6 bytes)
    const CALLEE: &[u8] = &[0x12, 0x06, b'D', b'L', b'1', b'A', b'B', b'C'];

    /// relay = "W1AW" (field 3, 4 bytes)
    const RELAY: &[u8] = &[0x1A, 0x04, b'W', b'1', b'A', b'W'];

    /// The full wire [`RadioLog::decoded`] decodes from
    fn wire() -> Vec<u8> {
        [Self::CALLER, Self::CALLEE, Self::RELAY].concat()
    }

    /// The canonical decoded [`RadioLog`] matching [`RadioLog::wire`]
    fn decoded() -> Self {
        Self {
            caller: Callsign("N0CALL".to_owned()),
            callee: Callsign("DL1ABC".to_owned()),
            relay: Some(Callsign("W1AW".to_owned())),
            witness: None,
        }
    }
}

/// A field whose own decoder takes no length decodes under a `lengthed` default, beside fields on
/// the default's `lengthed` decoder, in both drivers and at every chunk split
#[test]
fn des27_a_field_with_its_own_dec_is_not_lengthed() {
    RadioLog::decoded().assert_decodes_from(&RadioLog::wire());
}

/// A field with its own decoder still takes the default's width
#[test]
fn des27_a_field_with_its_own_dec_takes_the_defaults_exact() {
    #[rustfmt::skip]
    let wire = [
        RadioLog::CALLER,
        // callee = "DL1AB" (field 2, 5 bytes)
        &[0x12, 0x05, b'D', b'L', b'1', b'A', b'B'],
    ]
    .concat();
    RadioLog::assert_rejects_naming(&wire, &["`RadioLog::callee`", "size(exact = 6)"]);
}

/// A field on the default's decoder stays `lengthed` under a width of its own, which replaces the
/// default's
#[test]
fn des27_a_field_on_the_defaults_dec_keeps_lengthed_under_its_own_exact() {
    #[rustfmt::skip]
    let wire = [
        RadioLog::CALLER,
        RadioLog::CALLEE,
        // relay = "W1AW/P" (field 3, 6 bytes)
        &[0x1A, 0x06, b'W', b'1', b'A', b'W', b'/', b'P'],
    ]
    .concat();
    RadioLog::assert_rejects_naming(&wire, &["`RadioLog::relay`", "size(exact = 4)"]);
}

/// A field on the default's decoder that writes `lengthed` itself, as the default does, decodes
/// at the default's width
#[test]
fn des27_a_field_on_the_defaults_dec_may_write_lengthed_too() {
    #[rustfmt::skip]
    let wire = [
        RadioLog::CALLER,
        RadioLog::CALLEE,
        RadioLog::RELAY,
        // witness = "VE3XYZ" (field 4, 6 bytes)
        &[0x22, 0x06, b'V', b'E', b'3', b'X', b'Y', b'Z'],
    ]
    .concat();
    RadioLog {
        witness: Some(Callsign("VE3XYZ".to_owned())),
        ..RadioLog::decoded()
    }
    .assert_decodes_from(&wire);
}
