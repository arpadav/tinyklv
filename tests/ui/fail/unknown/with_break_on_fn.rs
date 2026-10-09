use tinyklv::prelude::*;

fn unknown_frame(_key: u8) -> impl Fn(&mut &[u8]) -> tinyklv::Result<Vec<u8>> {
    |input: &mut &[u8]| Ok(core::mem::take(input).to_vec())
}

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
    unknown(dec = unknown_frame),
    allow_unimplemented_encode,
    break_on = stop
)]
struct Reading {
    #[klv(key = 0x01, dec = tinyklv::dec::binary::u8)]
    id: u8,
}

fn main() {}
