use tinyklv::prelude::*;

#[derive(Debug, PartialEq)]
enum Payload {
    Count(u8),
}

#[derive(Klv)]
#[klv(
    key(dec = tinyklv::dec::binary::u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
    default(typ = u8, dec = tinyklv::dec::binary::u8, enc = *tinyklv::enc::binary::u8),
)]
struct Reading {
    #[klv(key = oneof(0x02 => Payload::Count(u8), 0x03 => self::Payload::Count(u8)))]
    payload: Option<Payload>,
}

fn main() {}
