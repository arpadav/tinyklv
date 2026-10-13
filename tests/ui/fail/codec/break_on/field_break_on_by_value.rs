use tinyklv::prelude::*;

fn ends_report(code: u8) -> BreakType {
    if code == 0 {
        BreakType::Proceed
    } else {
        BreakType::Done
    }
}

#[derive(Klv)]
#[klv(
    key(dec = tinyklv::dec::binary::u8),
    len(dec = tinyklv::dec::binary::u8_as_usize),
    allow_unimplemented_encode
)]
struct Report {
    #[klv(key = 0x01, dec = tinyklv::dec::binary::u8, break_on = ends_report)]
    code: u8,
}

fn main() {}
