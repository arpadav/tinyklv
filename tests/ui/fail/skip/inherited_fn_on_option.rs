use tinyklv::prelude::*;

fn is_zero(level: &u16) -> bool {
    *level == 0
}

#[derive(Klv)]
#[klv(
    key(dec = tinyklv::dec::binary::u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
    default(
        typ = u16,
        dec = tinyklv::dec::binary::be_u16,
        enc = *tinyklv::enc::binary::be_u16,
        skip_if = is_zero,
    ),
)]
struct Reading {
    #[klv(key = 0x01)]
    level: Option<u16>,
}

fn main() {}
