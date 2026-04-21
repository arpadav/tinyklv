// --------------------------------------------------
// local
// --------------------------------------------------
use tinyklv::prelude::*;
use tinyklv::Klv;

fn decode_u16_add_one(input: &mut &[u8]) -> tinyklv::Result<u16> {
    tinyklv::dec::binary::be_u16(input).map(|v| v + 1)
}

fn encode_u16_sub_one(input: &u16) -> Vec<u8> {
    tinyklv::enc::binary::be_u16(input.wrapping_sub(1))
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct CustomDecoders {
    #[klv(key = 0x01, dec = decode_u16_add_one, enc = encode_u16_sub_one)]
    adjusted: u16,
    #[klv(key = 0x02, dec = tinyklv::dec::binary::be_u8, enc = *tinyklv::enc::binary::u8)]
    plain: u8,
}

#[test]
/// Tests that a user-provided `dec` function (wire_value + 1) is applied during decode.
fn custom_decoder_applies_transform() {
    // Wire value is 0x0064 = 100; custom decoder adds 1 -> 101
    let data: &[u8] = &[0x01, 0x02, 0x00, 0x64, 0x02, 0x01, 0x07];
    let result = CustomDecoders::decode_value(&mut &data[..]).unwrap();
    assert_eq!(
        result.adjusted, 101,
        "custom decoder should add 1 to wire value"
    );
    assert_eq!(result.plain, 7);
}

#[test]
/// Verifies that the custom `enc`/`dec` pair compose as mutual inverses across a roundtrip.
fn custom_encoder_applies_inverse_transform() {
    let packet = CustomDecoders {
        adjusted: 101,
        plain: 7,
    };
    let encoded = packet.encode_value();
    let decoded = CustomDecoders::decode_value(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, packet);
}

#[test]
/// Tests the custom-encoder/decoder roundtrip across a spread of values including edge cases like `u16::MAX`.
fn custom_encoder_decode_roundtrip() {
    for adj in [1_u16, 100, 1000, u16::MAX] {
        let packet = CustomDecoders {
            adjusted: adj,
            plain: 0,
        };
        let encoded = packet.encode_value();
        let decoded = CustomDecoders::decode_value(&mut encoded.as_slice()).unwrap();
        assert_eq!(decoded.adjusted, adj);
    }
}

struct Transformer;
impl Transformer {
    fn decode_scaled(input: &mut &[u8]) -> tinyklv::Result<f32> {
        tinyklv::dec::binary::be_u16(input).map(|v| v as f32 / 100.0)
    }
    fn encode_scaled(input: &f32) -> Vec<u8> {
        tinyklv::enc::binary::be_u16((input * 100.0) as u16)
    }
}

#[derive(Klv, Debug)]
#[klv(
    stream = &[u8],
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct WithMethodDecoder {
    #[klv(key = 0x01, dec = Transformer::decode_scaled, enc = Transformer::encode_scaled)]
    scaled_value: f32,
}

#[test]
/// Tests that an associated-function decoder (`Transformer::decode_scaled`) applies the `u16 / 100.0` scaling.
fn method_decoder_scales_correctly() {
    // Wire value 0x01F4 = 500; scaled = 500 / 100 = 5.0
    let data: &[u8] = &[0x01, 0x02, 0x01, 0xF4];
    let result = WithMethodDecoder::decode_value(&mut &data[..]).unwrap();
    assert!((result.scaled_value - 5.0).abs() < 1e-6);
}

#[test]
/// Verifies roundtrip precision for an associated-function encoder/decoder pair over a fractional `f32` value.
fn method_encoder_roundtrip() {
    let packet = WithMethodDecoder {
        scaled_value: 12.34,
    };
    let encoded = packet.encode_value();
    let decoded = WithMethodDecoder::decode_value(&mut encoded.as_slice()).unwrap();
    assert!((decoded.scaled_value - packet.scaled_value).abs() < 0.01);
}
