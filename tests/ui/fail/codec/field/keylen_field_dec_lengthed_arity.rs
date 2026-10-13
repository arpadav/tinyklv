use tinyklv::{dec::binary as decb, prelude::*};

fn header(input: &mut &[u8]) -> tinyklv::Result<(u8, Framing)> {
    Ok((decb::u8(input)?, Framing::Sliced(decb::u8_as_usize(input)?)))
}

fn id(input: &mut &[u8]) -> tinyklv::Result<u16> {
    decb::be_u16(input)
}

#[derive(Klv)]
#[klv(keylen(dec = header), allow_unimplemented_encode)]
struct Reading {
    #[klv(key = 0x01, dec = id, size(lengthed, exact = 2))]
    id: u16,
}

fn main() {}
