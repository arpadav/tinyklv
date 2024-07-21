use tinyklv::Klv;

#[derive(Klv)]
#[key_encoder(func = key_encoder_v)]
#[key_decoder(func = key_decoder_v, fixed = true)]
#[len_encoder(func = len_encoder_v)]
#[len_decoder(func = len_decoder_v, fixed = false)]
// optional, as many as you like
#[default_encoder(typ = u8, func = serialize_u8)]
#[default_decoder(typ = u8, func = deserialize_u8)]
#[default_encoder(typ = Vec<f16>, func = serialize_u8)]
#[default_decoder(typ = Vec<f32>, func = deserialize_u8)]
pub struct MyStruct {
    #[key = b"\x01"]
    #[len = 2]
    #[encoder(func = key_encoder_vf)]
    #[decoder(func = key_decoder_vf, fixed = true)]
    pub BRUHHH: u8,
    
    #[key = b"\x02"]
    #[len = 1]
    #[encoder(func = key_encoder_vf)]
    #[decoder(func = key_decoder_vf, fixed = false)]
    pub b: u8,

    #[key = b"\x03"]
    #[len = 1]
    #[encoder(func = len_encoder_MEME)]
    #[decoder(func = len_decoder_MEME, fixed = true)]
    pub c: Vec<f32>,
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