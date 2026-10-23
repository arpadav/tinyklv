use tinyklv::{dec::binary as decb, prelude::*};

fn header(input: &mut &[u8]) -> tinyklv::Result<u8> {
    decb::u8(input)
}

#[derive(Klv)]
#[klv(keylen(dec = header), allow_unimplemented_encode)]
struct Reading {
    #[klv(key = 0x01, dec = decb::u8)]
    id: u8,
}

fn main() {}
