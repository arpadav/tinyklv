use tinyklv::prelude::*;

fn proceed(_: &u8) -> BreakType {
    BreakType::Proceed
}

fn done(_: &u8) -> BreakType {
    BreakType::Done
}

#[derive(Klv)]
#[klv(
    key(dec = tinyklv::dec::binary::u8),
    len(dec = tinyklv::dec::binary::u8_as_usize),
    allow_unimplemented_encode
)]
struct Reading {
    #[klv(key = 0x01, dec = tinyklv::dec::binary::u8, break_on = proceed, break_on = done)]
    id: u8,
}

fn main() {}
