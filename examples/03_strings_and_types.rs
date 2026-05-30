#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(clippy::unwrap_used)]
//! Example 03 - UTF-8 strings + variable-length fields
//!
//! Fixed-width fields use a plain `fn(&mut Stream) -> Result<T>` decoder
//! Variable-length fields (like strings) need the length to read a payload
//! bounded by the preceding `len` bytes - tinyklv surfaces this with the
//! `varlen = true` field attribute, which expects a length-parameterised
//! decoder of the form `fn(usize) -> impl FnMut(&mut Stream) -> Result<T>`
//!
//! Showcases:
//! * `varlen = true` for string payloads
//! * Mixing fixed-width `be_u32` and variable-length UTF-8 on one struct
//! * The split between `dec::binary` / `enc::binary` and `enc::string`
//!
//! See also: book Tutorial 03
use tinyklv::prelude::*;            // Klv proc-macro + traits
use tinyklv::dec::binary as decb;   // binary decoders
use tinyklv::dec::string as decs;   // string decoders
use tinyklv::enc::binary as encb;   // binary encoders
use tinyklv::enc::string as encs;   // string encoders

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"STATIONREG",
    key(dec = decb::u8,          enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
/// Weather-station registration transmitted over a telemetry bus
struct StationRegistration {
    #[klv(
        key = 0x01,
        dec = decb::be_u32,
        enc = *encb::be_u32,
    )]
    /// Fixed-width serial number (big-endian u32)
    serial: u32,

    #[klv(
        key = 0x02,
        varlen = true,
        dec = decs::to_string_utf8,
        enc = &encs::from_string_utf8,
    )]
    /// Variable-length UTF-8 region name; `varlen = true` selects the
    /// length-parameterised decoder signature
    region_name: String,

    #[klv(
        key = 0x03,
        varlen = true,
        dec = decs::to_string_utf8,
        enc = &encs::from_string_utf8,
    )]
    /// Variable-length UTF-8 station identifier
    station_id: String,
}

fn main() {
    // build
    let original = StationRegistration {
        serial:      0x00_AB_CD_12,
        region_name: String::from("North-Atlantic"),
        station_id:  String::from("WX-042"),
    };

    // encode - emits three KLV triples, the last two with a length prefix
    // computed from the UTF-8 byte length of the string
    let mut encoded = Vec::new();
    original.encode_value(&mut encoded);

    // decode - rebuilds the struct from the KLV triples
    let decoded = StationRegistration::decode_value(
        &mut encoded.as_slice(),
    ).unwrap();

    // assert - UTF-8 strings survive the round-trip unchanged
    assert_eq!(decoded, original);
}
