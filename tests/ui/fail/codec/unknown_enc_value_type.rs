use tinyklv::prelude::*;

fn unknown_frame(_key: u8) -> impl Fn(&mut &[u8]) -> tinyklv::Result<Vec<u8>> {
    |input: &mut &[u8]| Ok(core::mem::take(input).to_vec())
}

fn unknown_frame_enc(value: &[u16], out: &mut Vec<u8>) {
    out.extend(value.iter().map(|word| *word as u8));
}

#[derive(Klv)]
#[klv(
    key(dec = tinyklv::dec::binary::u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
    unknown(dec = unknown_frame, enc = unknown_frame_enc)
)]
struct Reading {
    #[klv(key = 0x01, dec = tinyklv::dec::binary::u8, enc = *tinyklv::enc::binary::u8)]
    id: u8,
    #[klv(unknown)]
    extra: Vec<(u8, Vec<u8>)>,
}

fn main() {}
