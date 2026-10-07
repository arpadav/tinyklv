use tinyklv::prelude::*;

#[derive(Klv)]
#[klv(
    key(dec = tinyklv::dec::binary::u8),
    len(dec = tinyklv::dec::binary::u8_as_usize),
    allow_unimplemented_encode
)]
struct Reading {
    #[klv(key = 0x01, dec = tinyklv::dec::binary::be_u64, size(hint = 8))]
    id: u64,
}

fn main() {}
