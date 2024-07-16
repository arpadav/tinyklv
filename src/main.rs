use tinyklv::Klv;

// #[klv(default_enc = (ty = u8, func = "serialize_u8"))]
#[derive(Klv)]
#[klv(key_dec = "key_decoder")]
#[klv(key_enc = "key_encoder")]
#[klv(len_dec = "len_decoder")]
#[klv(len_enc = "len_encoder")]
#[klv(default_dec(ty = u8, func = "deserialize_u8"))]
// #[klv_set_default_enc_for(ty = u8, func = "serialize_u8")]
pub struct MyStruct {
    #[klv(key = b"\x01")]
    #[klv(len = 2)]
    #[klv(dec = "serialize")]
    #[klv(enc = "deserialize")]
    pub BRUHHH: u8,
    
    #[klv(key = b"\x02")]
    #[klv(len = 1)]
    #[klv(dec = "serialize")]
    #[klv(enc = "deserialize")]
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