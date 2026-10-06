use tinyklv::prelude::*;

#[derive(Klv)]
#[klv(keylen, allow_unimplemented_encode)]
struct Reading {
    #[klv(key = 0x01, dec = tinyklv::dec::binary::u8)]
    id: u8,
}

fn main() {}
