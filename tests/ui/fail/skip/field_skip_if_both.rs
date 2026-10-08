use tinyklv::prelude::*;

#[derive(Klv)]
#[klv(
    key(dec = tinyklv::dec::binary::u8),
    len(dec = tinyklv::dec::binary::u8_as_usize),
    allow_unimplemented_encode
)]
struct Reading {
    #[klv(
        key = 0x01,
        dec = tinyklv::dec::binary::be_u32,
        skip_if_default,
        skip_if = Reading::is_unset,
    )]
    id: u32,
}

impl Reading {
    fn is_unset(id: &u32) -> bool {
        *id == u32::MAX
    }
}

fn main() {}
