use tinyklv::prelude::*;

fn unknown_frame(_key: u8) -> impl Fn(&mut &[u8]) -> tinyklv::Result<Vec<u8>> {
    |input: &mut &[u8]| Ok(core::mem::take(input).to_vec())
}

fn unknown_frame_enc(value: &Vec<u8>, out: &mut Vec<u8>) {
    out.extend_from_slice(value);
}

#[derive(Klv)]
#[klv(
    key(dec = tinyklv::dec::binary::u8),
    len(dec = tinyklv::dec::binary::u8_as_usize),
    unknown(dec = unknown_frame, enc = unknown_frame_enc, enc = unknown_frame_enc),
    allow_unimplemented_encode
)]
struct Reading {
    #[klv(key = 0x01, dec = tinyklv::dec::binary::u8)]
    id: u8,
}

fn main() {}
