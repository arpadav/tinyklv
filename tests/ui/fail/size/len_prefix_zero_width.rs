use tinyklv::prelude::*;

#[derive(Klv)]
#[klv(
    key(dec = tinyklv::dec::binary::u8, enc = tinyklv::enc::binary::u8),
    len(
        dec = tinyklv::dec::binary::u8_as_usize,
        enc = tinyklv::enc::binary::u8_from_usize,
        size(exact = 0),
    ),
)]
struct Reading {
    #[klv(
        key = 0x01,
        dec = tinyklv::dec::string::to_string_utf8,
        enc = tinyklv::enc::string::from_string_utf8,
        size(lengthed),
    )]
    name: String,
}

fn main() {}
