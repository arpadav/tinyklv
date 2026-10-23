use tinyklv::prelude::*;

#[derive(Klv)]
#[klv(
    key(dec = tinyklv::dec::binary::u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
    default(typ = u8, dec = tinyklv::dec::binary::u8, enc = *tinyklv::enc::binary::u8),
)]
struct Reading {
    #[klv(key = 0x10)]
    count: u8,
    #[klv(key = 16)]
    level: u8,
}

fn main() {}
