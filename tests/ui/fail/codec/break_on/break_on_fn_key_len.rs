use tinyklv::prelude::*;

fn stop(key: u8, _len: usize) -> BreakType {
    if key == 0xFF {
        BreakType::Done
    } else {
        BreakType::Proceed
    }
}

#[derive(Klv)]
#[klv(
    key(dec = tinyklv::dec::binary::u8),
    len(dec = tinyklv::dec::binary::u8_as_usize),
    break_on = stop,
    allow_unimplemented_encode
)]
struct Reading {
    #[klv(key = 0x01, dec = tinyklv::dec::binary::u8)]
    id: u8,
}

fn main() {}
