use tinyklv::{dec::binary as decb, prelude::*};

/// Its `be_u16` name implies a 2-byte value; a one-token path spans alike on every toolchain
fn be_u16(value: u16, out: &mut Vec<u8>) {
    out.extend_from_slice(&value.to_be_bytes());
}

#[derive(Klv)]
#[klv(
    key(dec = decb::u8, enc = tinyklv::enc::binary::u8),
    len(
        dec = decb::u8_as_usize,
        enc = tinyklv::enc::binary::u8_from_usize,
        size(exact = 0),
    ),
)]
struct Reading {
    #[klv(key = 0x01, dec = decb::be_u16, enc = *be_u16)]
    level: u16,
}

fn main() {}
