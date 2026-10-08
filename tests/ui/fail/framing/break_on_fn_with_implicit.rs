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
    default(typ = u16, dec = tinyklv::dec::binary::be_u16, size(implicit, exact = 2)),
    allow_unimplemented_encode,
    break_on = stop
)]
struct Reading {
    #[klv(key = 0x01, dec = tinyklv::dec::binary::u8)]
    id: u8,
    #[klv(key = 0x02)]
    level: u16,
}

fn main() {}
