use tinyklv::prelude::*;

#[derive(Klv)]
#[klv(
    key(dec = tinyklv::dec::binary::u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::u8_as_usize),
    allow_unimplemented_decode
)]
struct Reading {
    #[klv(key = 0x01, enc = *tinyklv::enc::binary::u8)]
    id: u8,
}

fn main() {}
