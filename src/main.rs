use tinyklv::Klv;

// key / len encoder / decoder's will always
// take the entire input buffer
// - e.g. VariableDecoder

// value encoder / decoder's will always
// take a fixed slice of the input buffer,
// determined by the value of the `len` field
// - e.g. FixedDecoder

#[derive(Klv)]
#[key_encoder(func = key_encoder_example)]
#[key_decoder(func = key_decoder_example)]
#[len_encoder(func = len_encoder_example)]
#[len_decoder(func = len_decoder_example)]
// optional, as many as you like
#[default_encoder(typ = u8, func = serialize_u8)]
#[default_decoder(typ = u8, func = deserialize_u8)]
#[default_encoder(typ = Vec<f32>, func = serialize_u8)]
#[default_decoder(typ = Vec<f64>, func = deserialize_u8)]
pub struct MyStruct {
    #[key = b"\x01"]
    #[len = 2]
    #[encoder(func = key_encoder_vf)]   // input: u8, output: &[u8; 2]
    #[decoder(func = key_decoder_vf)]   // input: &[u8; 2], output: u8
    pub BRUHHH: u8,
    
    #[key = b"\x02"]
    #[len = 1]
    #[encoder(func = key_encoder_vf)]   // input: u8, output: &[u8; 1]
    #[decoder(func = key_decoder_vf)]   // input: &[u8; 1], output: u8
    pub b: u8,

    #[key = b"\x03"]
    #[len = 1]
    #[encoder(func = len_encoder_MEME)] // input: Vec<f32>, output: &[u8; 1]
    #[decoder(func = len_decoder_MEME)] // input: &[u8; 1], output: Vec<f32>
    pub c: Vec<f32>,
}

// should do
// impl LenDecoder<T> for MyStruct {
//     fn len_decoder(&self, input: &[u8]) -> nom::IResult<&[u8], T> {
//         #attr_func (input)
//     }
// }

// should do:
// expand.rs
// other files, similar to thiserror
fn main() {
    let size = b"\x01".len();
}

fn key_decoder_example(input: &[u8]) -> Option<&[u8]> {
    Some(input)
}

fn key_encoder_example(input: &[u8]) -> Option<&[u8]> {
    Some(input)
}

fn len_decoder_example(input: &[u8]) -> Option<&[u8]> {
    Some(input)
}

fn len_encoder_example(input: &[u8]) -> Option<&[u8]> {
    Some(input)
}

// fn serialize_u8(input: u8) -> Option<[u8]> {
//     Some([input.clone()])
// }

fn deserialize_u8(input: &[u8]) -> Option<u8> {
    Some(input[0])
}