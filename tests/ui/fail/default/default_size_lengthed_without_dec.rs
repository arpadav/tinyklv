use tinyklv::prelude::*;

#[derive(Klv)]
#[klv(
    key(dec = tinyklv::dec::binary::u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
    default(typ = u16, enc = *tinyklv::enc::binary::be_u16, size(lengthed)),
)]
struct Reading {
    #[klv(key = 0x01, dec = tinyklv::dec::binary::be_u16)]
    level: u16,
}

fn main() {}
