use tinyklv::prelude::*;

fn is_last(code: &u8) -> bool {
    *code == 0
}

#[derive(Klv)]
#[klv(
    key(dec = tinyklv::dec::binary::u8),
    len(dec = tinyklv::dec::binary::u8_as_usize),
    allow_unimplemented_encode
)]
struct Report {
    #[klv(key = 0x01, dec = tinyklv::dec::binary::u8, break_on = is_last)]
    code: u8,
}

fn main() {}
