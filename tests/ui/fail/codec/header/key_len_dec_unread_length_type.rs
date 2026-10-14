use tinyklv::prelude::*;

fn len_u128(input: &mut &[u8]) -> tinyklv::Result<u128> {
    tinyklv::dec::binary::be_u128(input)
}

fn skip(_key: u8) -> impl Fn(&mut &[u8]) -> tinyklv::Result<u8> {
    tinyklv::dec::binary::u8
}

#[derive(Klv)]
#[klv(
    key(dec = tinyklv::dec::binary::u8),
    len(dec = len_u128),
    unknown(dec = skip),
    allow_unimplemented_encode,
)]
struct Reading {
    #[klv(key = 0x01, dec = tinyklv::dec::binary::u8, size(implicit))]
    id: u8,
}

fn main() {}
