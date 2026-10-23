use tinyklv::prelude::*;

#[derive(Debug, PartialEq)]
enum Payload {
    Count(u8),
    Name(String),
}

#[derive(Klv)]
#[klv(
    key(dec = tinyklv::dec::binary::u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
    default(typ = u8, dec = tinyklv::dec::binary::u8, enc = *tinyklv::enc::binary::u8),
)]
struct Reading {
    #[klv(key = oneof(0x01 => Payload::Count(u8), 0x02 => Payload::Name(String)))]
    payload: Option<Payload>,
}

fn main() {}
