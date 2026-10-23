use tinyklv::{dec::binary as decb, prelude::*};

fn header(input: &mut &[u8]) -> tinyklv::Result<(u8, Framing)> {
    Ok((decb::u8(input)?, Framing::Inline))
}

fn id(_input: &mut &[u8]) -> Result<u8, ()> {
    Err(())
}

#[derive(Klv)]
#[klv(keylen(dec = header), allow_unimplemented_encode)]
struct Reading {
    #[klv(key = 0x01, dec = id)]
    id: u8,
}

fn main() {}
