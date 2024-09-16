#![allow(klv_unimplemented_decode)]

use tinyklv::Klv;
use tinyklv::prelude::*;

struct InnerValue {}

fn ex01_encoder(input: &InnerValue) -> Vec<u8> {
    return vec![0x65, 0x66, 0x67, 0x68];
}

fn ex02_encoder(input: &InnerValue) -> Vec<u8> {
    return String::from("Y2K").into_bytes();
}

impl EncodeValue<Vec<u8>> for InnerValue {
    fn encode_value(&self) -> Vec<u8> {
        return String::from("KLV").to_lowercase().into_bytes();
    }
}

#[derive(Klv)]
#[klv(
    stream = &[u8],
    sentinel = 0x00,
    key(enc = tinyklv::codecs::binary::enc::u8,
        dec = tinyklv::codecs::binary::dec::u8),
    len(enc = tinyklv::codecs::binary::enc::u8_from_usize,
        dec = tinyklv::codecs::binary::dec::u8),
)]
// #[klv(allow_unimplemented_decode)]
// #[klv(allow_unimplemented_encode)]
struct MyStruct {
    #[klv(key = 0x07, enc = ex01_encoder)]
    example_one: InnerValue,

    #[klv(key = 0x0A, enc = ex02_encoder)]
    example_two: InnerValue,

    #[klv(key = 0x8A, enc = InnerValue::encode_value)]
    example_three: InnerValue,
}

#[test]
fn test0() {
    let my_struct_encoded = MyStruct{
        example_one: InnerValue {},
        example_two: InnerValue {},
        example_three: InnerValue {},
    }.encode();

    assert_eq!(my_struct_encoded, vec![
        0x00,               // sentinel
        0x10,               // total length
    
        // example 1
        0x07,               // example 1 key
        0x04,               // example 1 length
                            // example 1 value 
        0x65, 0x66, 0x67, 0x68,
    
        // example 2
        0x0A,               // example 2 key
        0x03,               // example 2 length
        0x59, 0x32, 0x4B,   // example 2 value
    
        // example 3
        0x8A,               // example 3 key
        0x03,               // example 3 length
        0x6B, 0x6C, 0x76,   // example 3 value
    ]);
}

fn main() {}