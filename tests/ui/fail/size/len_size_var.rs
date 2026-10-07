use tinyklv::prelude::*;

#[derive(Klv)]
#[klv(
    key(dec = tinyklv::dec::binary::u8),
    len(dec = tinyklv::dec::binary::u8_as_usize, size(var)),
    allow_unimplemented_encode
)]
struct Reading {
    #[klv(key = 0x01, dec = tinyklv::dec::binary::u8)]
    id: u8,
}

fn main() {}
