use tinyklv::{dec::binary as decb, prelude::*};

fn header(input: &mut &[u8]) -> tinyklv::Result<(u8, Framing)> {
    Ok((decb::u8(input)?, Framing::Sliced(decb::u8_as_usize(input)?)))
}

#[derive(Klv)]
#[klv(keylen(dec = header, encoder = header_enc))]
struct Reading {
    #[klv(key = 0x01, dec = decb::u8, enc = *tinyklv::enc::binary::u8)]
    id: u8,
}

fn main() {}
