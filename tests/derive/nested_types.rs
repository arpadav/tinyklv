//! User-defined nested-type field tests for `#[derive(Klv)]`
//!
//! Tests a `WithNestedType` struct where one field holds a user-defined
//! `Point` struct (x/y as `i16`) that manually implements `DecodeValue` and
//! `EncodeValue`. Covers known-value decode, roundtrip, origin point, and
//! signed-integer extreme coordinates (`i16::MIN`/`MAX`)
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use tinyklv::dec::binary as decb;
use tinyklv::enc::binary as encb;
use tinyklv::prelude::*;

#[derive(Debug, PartialEq, Clone)]
struct Point {
    x: i16,
    y: i16,
}
impl tinyklv::DecodeValue<&[u8]> for Point {
    fn decode_value(input: &mut &[u8]) -> tinyklv::Result<Self> {
        let x = decb::be_i16(input)?;
        let y = decb::be_i16(input)?;
        Ok(Point { x, y })
    }
}
impl tinyklv::EncodeValue for Point {
    fn encode_value(&self, out: &mut Vec<u8>) {
        encb::be_i16(self.x, out);
        encb::be_i16(self.y, out);
    }
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
    trait_fallback,
)]
struct WithNestedType {
    #[klv(
        key = 0x01,
        dec = decb::be_u16,
        enc = *encb::be_u16,
    )]
    id: u16,

    #[klv(key = 0x02)]
    location: Point,
}

#[test]
/// Tests that a user-defined `Point` struct (implementing `DecodeValue`) decodes correctly as a nested field
fn decode_nested_type() {
    let data: &[u8] = &[
        0x01, 0x02, 0x00, 0x2A, // id = 42
        0x02, 0x04, // key=0x02, len=4
        0x00, 0x0A, 0xFF, 0xF6, // x=10, y=-10 (as BE i16)
    ];
    let result = WithNestedType::decode_value(&mut &data[..]).unwrap();
    assert_eq!(result.id, 42);
    assert_eq!(result.location.x, 10);
    assert_eq!(result.location.y, -10);
}

#[test]
/// Verifies encode/decode roundtrip when a field is a user-defined nested struct with signed coordinates
fn encode_nested_type_roundtrip() {
    let original = WithNestedType {
        id: 99,
        location: Point { x: 100, y: -200 },
    };
    let mut encoded = Vec::new();
    original.encode_value(&mut encoded);
    let decoded = WithNestedType::decode_value(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original);
}

#[test]
/// Tests nested-type roundtrip with the zero/origin case `Point { x: 0, y: 0 }`
fn nested_type_origin_point() {
    let original = WithNestedType {
        id: 0,
        location: Point { x: 0, y: 0 },
    };
    let mut encoded = Vec::new();
    original.encode_value(&mut encoded);
    let decoded = WithNestedType::decode_value(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original);
}

#[test]
/// Tests nested-type roundtrip at signed-integer extremes (`i16::MIN`/`MAX`) to exercise sign-boundary encoding
fn nested_type_extreme_coordinates() {
    let original = WithNestedType {
        id: u16::MAX,
        location: Point {
            x: i16::MIN,
            y: i16::MAX,
        },
    };
    let mut encoded = Vec::new();
    original.encode_value(&mut encoded);
    let decoded = WithNestedType::decode_value(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original);
}
