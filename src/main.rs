use tinyklv::Klv;

// key / len encoder / decoder's will always
// take the entire input buffer
// - e.g. VariableDecoder

// value encoder / decoder's will always
// take a fixed slice of the input buffer,
// determined by the value of the `len` field
// - e.g. FixedDecoder

// #[derive(Klv)]
// // #[seek = b"\x00"]
// #[key_encoder(func = key_encoder_example)]
// #[key_decoder(func = key_decoder_example)]
// #[len_encoder(func = len_encoder_example)]
// #[len_decoder(func = len_decoder_example)]
// // optional, as many as you like
// // #[default_encoder(typ = u8, func = serialize_u8)]
// // #[default_decoder(typ = u8, func = deserialize_u8)]
// #[default_encoder(typ = Vec<f32>, func = serialize_vec_f32)]
// // #[default_decoder(typ = Vec<f64>, func = deserialize_u8)]
// pub struct MyStruct {
//     #[key = b"\x01"]
//     #[len = 2]
//     #[encoder(func = serialize_u8)]   // input: u8, output: &[u8; 2]
//     #[decoder(func = deserialize_u8)]   // input: &[u8; 2], output: u8
//     pub BRUHHH: u8,

//     #[key = b"\x02"]
//     #[len = 1]
//     #[encoder(func = serialize_u8)]   // input: u8, output: &[u8; 2]
//     #[decoder(func = deserialize_u8)]   // input: &[u8; 2], output: u8
//     pub b: u8,

//     #[key = b"\x03"]
//     #[len = 1]
//     // #[encoder(func = serialize_vec_f32)] // input: Vec<f32>, output: &[u8; 1]
//     #[decoder(func = deserialize_vec_f32)] // input: &[u8; 1], output: Vec<f32>
//     pub someting: Vec<f32>,
// }

// key/len xcoder, fixed ALWAYS false
// variants: fixed ALWAYS true
//
// TODO: think about include_self terminology.
//
// include self NEVER in key/len, optional in variant

#[derive(Klv)]
#[klv(
    stream = u8, // input is a slice/vec of this
                 // non-configurable, for now. but make
                 // type-agnostic.
    
    sentinel = b"\x01", // sentinal / keys always slice / vec of dtype stream

    // type defaults to stream type, any length
    key(enc = someting, dec = someting2), // both required
    // type will ALWAYS be usize
    len(enc = lsometing, dec = lsometing2), // both required
    default(ty = u16, enc = this, dec = that), // ty required, enc OR dec required
    default(ty = f32, enc = foo, dec = bar),  // ty required, enc OR dec required
    default(ty = Vec<f64>, enc = me), // ty required, enc OR dec required
)]
struct Bruh {
    #[klv(
        key = b"\x02",
        len = 3,
        enc = my_str_enc,
        dec = my_str_dec,
    )]
    val: String,

    #[klv(
        key = b"\x03",
        len = 3,
        enc = my_str_enc,
        dec = my_str_dec,
    )]
    val2: String,

    #[klv(
        key = b"\x04",
        len = 3,
        enc = my_str_enc,
        dec = my_str_dec,
    )]
    another_val: String,

    #[klv(
        key = b"\x05",
        len = 3,
        enc = my_str_enc,
        dec = my_str_dec,
    )]
    yet_another_val: String,

    #[klv(
        key = b"\x06",
        len = 3,
        enc = my_str_enc,
        dec = my_str_dec,
    )]
    yet_yet_another_val: String,
}


// should do:
// expand.rs
// other files, similar to thiserror

// pub struct MyStruct {
//     pub BRUHHH: u8,
//     pub b: u8,
//     pub c: Vec<f32>,
// }


// impl tinyklv::KeyDecoder<Vec<u8>> for MyStruct {
//     fn key_decode<'a>(&self, input: &'a [u8]) -> nom::IResult<&'a [u8], Vec<u8>> {
//         match key_decoder_example(input) {
//             Ok((i, o)) => Ok((i, o.into())),
//             Err(e) => Err(e),
//         }
//     }
// }
// impl tinyklv::KeyEncoder<Vec<u8>> for MyStruct {
//     fn key_encode(&self, input: Vec<u8>) -> Vec<u8> {
//         key_encoder_example(input)
//     }
// }
// impl tinyklv::LenDecoder<Vec<u8>> for MyStruct {
//     fn len_decode<'a>(&self, input: &'a [u8]) -> nom::IResult<&'a [u8], Vec<u8>> {
//         match len_decoder_example(input) {
//             Ok((i, o)) => Ok((i, o.into())),
//             Err(e) => Err(e),
//         }
//     }
// }
// impl tinyklv::LenEncoder<Vec<u8>> for MyStruct {
//     fn len_encode(&self, input: Vec<u8>) -> Vec<u8> {
//         len_encoder_example(input)
//     }
// }

fn main() {
    let size = b"\x01".len();
    println!("size: {}", size);
}

fn key_decoder_example(input: &[u8]) -> nom::IResult<&[u8], &[u8]> {
    Ok((input, input))
}

fn key_encoder_example(input: Vec<u8>) -> Vec<u8> {
    input
}

fn len_decoder_example(input: &[u8]) -> nom::IResult<&[u8], &[u8]> {
    Ok((input, input))
}

fn len_encoder_example(input: Vec<u8>) -> Vec<u8> {
    input
}

fn serialize_u8(input: u8) -> Vec<u8> {
    vec![input]
}

fn deserialize_u8(input: &[u8]) -> nom::IResult<&[u8], u8> {
    Ok((input[1..].as_ref(), input[0]))
}


fn serialize_vec_f32(input: Vec<f32>) -> Vec<u8> {
    input
        .iter()
        .flat_map(|x| x.to_le_bytes().to_vec())
        .collect::<Vec<u8>>()
}

fn deserialize_vec_f32(input: &[u8]) -> nom::IResult<&[u8], Vec<f32>> {
    Ok((input, Vec::new()))
}