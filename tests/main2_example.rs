#![allow(dead_code)]
// #[prelude_import]
use std::prelude::rust_2021::*;
#[macro_use]
extern crate std;
use tinyklv::Klv;
use tinyklv::prelude::*;

struct InnerValue {}
fn ex01_encoder(_: &InnerValue) -> Vec<u8> {
    return vec![0x65, 0x66, 0x67, 0x68];
}
fn ex02_encoder(_: &InnerValue) -> Vec<u8> {
    return String::from("Y2K").into_bytes();
}
impl tinyklv::EncodeValue<Vec<u8>> for InnerValue {
    fn encode_value(&self) -> Vec<u8> {
        return String::from("KLV").to_lowercase().into_bytes();
    }
}
fn my_decoder_key(
    x: &mut &[u8],
) -> tinyklv::Result<u8> {
    tinyklv::codecs::binary::dec::u8(x)
}

#[derive(Klv)]
#[klv(
    stream = &[u8],
    sentinel = b"\x00",
    key(enc = "tinyklv::codecs::binary::enc::u8", dec = "my_decoder_key"),
    len(
        enc = "tinyklv::codecs::binary::enc::u8_from_usize",
        dec = "tinyklv::codecs::binary::dec::u8"
    ),
    default(typ = "u64", dec = "tinyklv::codecs::binary::dec::be_u64"),
    allow_unimplemented_decode,
)]
struct MyStruct {
    #[klv(key = 0x07, enc = "ex01_encoder")]
    example_one: InnerValue,
    #[klv(key = 0x0A, enc = ex02_encoder)]
    example_two: InnerValue,
    #[klv(key = 0x8A, enc = "InnerValue::encode_value")]
    example_three: InnerValue,
}

fn main() {}
