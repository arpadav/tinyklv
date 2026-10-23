use tinyklv::{dec::binary as decb, enc::binary as encb, prelude::*};

fn len_enc(len: u8, out: &mut Vec<u8>) {
    out.push(len);
}

#[derive(Klv)]
#[klv(
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = len_enc),
)]
struct Reading {
    #[klv(key = 0x01, dec = decb::u8, enc = *encb::u8)]
    id: u8,
    #[klv(key = 0x02, dec = decb::be_u16, enc = *encb::be_u16)]
    level: u16,
}

fn main() {}
