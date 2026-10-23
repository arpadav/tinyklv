use tinyklv::{dec::binary as decb, enc::binary as encb, prelude::*};

fn header(input: &mut &[u8]) -> tinyklv::Result<(u8, Framing)> {
    Ok((decb::u8(input)?, Framing::Sliced(decb::u8_as_usize(input)?)))
}

fn header_enc((key, len): (u8, usize), out: &mut Vec<u8>) {
    out.push(key);
    encb::u8_from_usize(len, out);
}

#[derive(Klv)]
#[klv(keylen(dec = header, enc = header_enc, size(exact = 2)))]
struct Reading {
    #[klv(key = 0x01, dec = decb::u8, enc = *encb::u8)]
    id: u8,
}

fn main() {}
