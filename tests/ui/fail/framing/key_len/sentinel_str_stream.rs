use tinyklv::prelude::*;

fn key(_: &mut &str) -> tinyklv::Result<u8> {
    Ok(0x01)
}

fn len(_: &mut &str) -> tinyklv::Result<usize> {
    Ok(1)
}

fn id(_: &mut &str) -> tinyklv::Result<u8> {
    Ok(7)
}

#[derive(Klv)]
#[klv(
    stream = &str,
    sentinel = "KLV",
    key(dec = key),
    len(dec = len),
    allow_unimplemented_encode
)]
struct Reading {
    #[klv(key = 0x01, dec = id)]
    id: u8,
}

fn main() {}
