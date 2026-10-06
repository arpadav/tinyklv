use tinyklv::{dec::binary as decb, enc::binary as encb, prelude::*};

fn id(input: &mut &[u8]) -> tinyklv::Result<u16> {
    decb::be_u16(input)
}

#[derive(Klv)]
#[klv(
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct Reading {
    #[klv(key = 0x01, dec = id, enc = *encb::u8)]
    id: u8,
}

fn main() {}
