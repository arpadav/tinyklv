// --------------------------------------------------
// local
// --------------------------------------------------
use tinyklv::prelude::*;
use tinyklv::Klv;

#[derive(Debug, PartialEq, Clone)]
struct Point {
    x: i16,
    y: i16,
}
impl tinyklv::DecodeValue<&[u8]> for Point {
    fn decode_value(input: &mut &[u8]) -> tinyklv::Result<Self> {
        let x = tinyklv::dec::binary::be_i16(input)?;
        let y = tinyklv::dec::binary::be_i16(input)?;
        Ok(Point { x, y })
    }
}
impl tinyklv::EncodeValue<Vec<u8>> for Point {
    fn encode_value(&self) -> Vec<u8> {
        let mut v = tinyklv::enc::binary::be_i16(self.x);
        v.extend(tinyklv::enc::binary::be_i16(self.y));
        v
    }
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct WithNestedType {
    #[klv(key = 0x01, dec = tinyklv::dec::binary::be_u16, enc = &tinyklv::enc::binary::be_u16)]
    id: u16,
    #[klv(key = 0x02, dec = Point::decode_value, enc = Point::encode_value)]
    location: Point,
}

#[test]
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
fn encode_nested_type_roundtrip() {
    let original = WithNestedType {
        id: 99,
        location: Point { x: 100, y: -200 },
    };
    let encoded = original.encode_value();
    let decoded = WithNestedType::decode_value(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn nested_type_origin_point() {
    let original = WithNestedType {
        id: 0,
        location: Point { x: 0, y: 0 },
    };
    let encoded = original.encode_value();
    let decoded = WithNestedType::decode_value(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn nested_type_extreme_coordinates() {
    let original = WithNestedType {
        id: u16::MAX,
        location: Point {
            x: i16::MIN,
            y: i16::MAX,
        },
    };
    let encoded = original.encode_value();
    let decoded = WithNestedType::decode_value(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original);
}
