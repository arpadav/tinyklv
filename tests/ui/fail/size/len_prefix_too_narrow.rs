use tinyklv::{dec::binary as decb, prelude::*};
use winnow::{Parser, token::take};

/// 256 bytes, one more than a one-byte length prefix encodes
#[derive(Debug)]
struct Digest([u8; 256]);

fn digest_dec(input: &mut &[u8]) -> tinyklv::Result<Digest> {
    let mut digest = [0u8; 256];
    digest.copy_from_slice(take(256usize).parse_next(input)?);
    Ok(Digest(digest))
}

fn digest_enc(digest: &Digest, out: &mut Vec<u8>) {
    out.extend_from_slice(&digest.0);
}

#[derive(Klv)]
#[klv(
    key(dec = decb::u8, enc = tinyklv::enc::binary::u8),
    len(
        dec = decb::u8_as_usize,
        enc = tinyklv::enc::binary::u8_from_usize,
        size(exact = 1),
    ),
)]
struct Reading {
    #[klv(key = 0x01, dec = digest_dec, enc = digest_enc, size(exact = 256))]
    digest: Digest,
}

fn main() {}
