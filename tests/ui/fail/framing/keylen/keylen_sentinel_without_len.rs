use tinyklv::{dec::binary as decb, prelude::*};

fn header(input: &mut &[u8]) -> tinyklv::Result<(u8, Framing)> {
    Ok((decb::u8(input)?, Framing::Sliced(decb::u8_as_usize(input)?)))
}

#[derive(Klv)]
#[klv(keylen(dec = header), sentinel = b"\x06", allow_unimplemented_encode)]
struct Reading {
    #[klv(key = 0x01, dec = decb::u8)]
    id: u8,
}

#[derive(Klv)]
#[klv(keylen(dec = header), sentinel(value = b"\x06"), allow_unimplemented_encode)]
struct Report {
    #[klv(key = 0x01, dec = decb::u8)]
    id: u8,
}

fn main() {}
