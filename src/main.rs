use tinyklv::Klv;
use tinyklv::prelude::*;

#[derive(Klv)]
#[klv(decoder(Key, func = "key_decoder"), encoder(Key, func = "key_encoder"))]
#[klv(Key::Decoder, func = "key_decoder", fixed = false)]
#[klv(Key::Encoder, func = "key_encoder")]
#[klv(Len::Decoder, func = "len_decoder", fixed = false)]
#[klv(Len::Encoder, func = "len_encoder")]
#[klv(DefaultType::Decoder, ty = u8, func = "deserialize_u8")]
pub struct MyStruct {
    #[klv(key = b"\x01")]
    #[klv(len = 2)]
    #[klv(Val::Decoder, func = "serialize")]
    #[klv(Val::Encoder, func = "deserialize")]
    pub BRUHHH: u8,
    
    #[klv(key = b"\x02")]
    #[klv(len = 1)]
    #[klv(Val::Decoder, func = "serialize")]
    #[klv(Val::Encoder, func = "deserialize")]
    pub b: u8,
}

fn main() {}

fn key_decoder(input: &[u8]) -> Option<&[u8]> {
    Some(input)
}

fn key_encoder(input: &[u8]) -> Option<&[u8]> {
    Some(input)
}

fn len_decoder(input: &[u8]) -> Option<&[u8]> {
    Some(input)
}

fn len_encoder(input: &[u8]) -> Option<&[u8]> {
    Some(input)
}

// fn serialize_u8(input: u8) -> Option<[u8]> {
//     Some([input.clone()])
// }

fn deserialize_u8(input: &[u8]) -> Option<u8> {
    Some(input[0])
}