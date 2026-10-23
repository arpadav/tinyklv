use tinyklv::prelude::*;

#[derive(Klv)]
#[klv(
    sentinel(
        value = b"READ",
        len(dec = tinyklv::dec::binary::be_u16_as_usize),
        len(dec = tinyklv::dec::binary::u8_as_usize),
    ),
    key(dec = tinyklv::dec::binary::u8),
    len(dec = tinyklv::dec::binary::u8_as_usize),
    allow_unimplemented_encode
)]
struct Reading {
    #[klv(key = 0x01, dec = tinyklv::dec::binary::u8)]
    seq: u8,
}

fn main() {}
