use tinyklv::{dec::binary as decb, prelude::*};

fn header(input: &mut &[u8]) -> tinyklv::Result<(u8, Framing)> {
    Ok((decb::u8(input)?, Framing::Sliced(decb::u8_as_usize(input)?)))
}

#[derive(Klv)]
#[klv(keylen(dec = header), allow_unimplemented_encode)]
struct Reading {
    #[klv(key = 0x01, dec = decb::be_u16, size(implicit))]
    level: u16,
}

fn main() {}
