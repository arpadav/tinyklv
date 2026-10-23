use tinyklv::prelude::*;

#[derive(Debug, PartialEq)]
enum Payload {
    Count(u8),
    Level(u16),
}

#[derive(Klv)]
#[klv(
    key(dec = tinyklv::dec::binary::u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
    default(typ = u8, dec = tinyklv::dec::binary::u8, enc = *tinyklv::enc::binary::u8),
    default(typ = u16, dec = tinyklv::dec::binary::be_u16, enc = *tinyklv::enc::binary::be_u16),
)]
struct Reading {
    #[klv(key = oneof(0x02 => Payload::Count(u8), 0x02 => Payload::Level(u16)))]
    payload: Option<Payload>,
}

fn main() {}
