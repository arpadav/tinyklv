use tinyklv::prelude::*;

#[derive(Klv)]
#[klv(
    key(dec = tinyklv::dec::binary::u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
    allow_unimplemented_encode,
)]
struct Reading {
    #[klv(key = 0x01)]
    level: Option<u16>,
}

fn main() {}
