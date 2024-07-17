use tinyklv::Klv;

#[derive(Klv)]
#[key_encoder(func = "key_encoder")]
#[key_decoder(func = "key_decoder", fixed = false)]
#[len_encoder(func = "len_encoder")]
#[len_decoder(func = "len_decoder", fixed = false)]
// optional, as many as you like
#[default_encoder(ty = u8, func = "serialize_u8")]
#[default_decoder(ty = u8, func = "deserialize_u8")]
pub struct MyStruct {
    #[key(b"\x01")]
    #[len(2)]
    #[encoder(func = "key_encoder")]
    #[decoder(func = "key_decoder", fixed = false)]
    pub BRUHHH: u8,
    
    #[key(b"\x02")]
    #[len(1)]
    #[encoder(func = "key_encoder")]
    #[decoder(func = "key_decoder", fixed = false)]
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