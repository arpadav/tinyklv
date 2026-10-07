use tinyklv::{dec::binary as decb, prelude::*};

fn header(input: &mut &[u8]) -> tinyklv::Result<(u8, Framing)> {
    Ok((decb::u8(input)?, Framing::Sliced(decb::u8_as_usize(input)?)))
}

fn stop(key: u8, _len: usize) -> BreakType {
    if key == 0xFF {
        BreakType::Done
    } else {
        BreakType::Proceed
    }
}

#[derive(Klv)]
#[klv(keylen(dec = header), allow_unimplemented_encode, break_on = stop)]
struct Reading {
    #[klv(key = 0x01, dec = decb::u8)]
    id: u8,
}

fn main() {}
