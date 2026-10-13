use tinyklv::{dec::binary as decb, enc::binary as encb, prelude::*};

fn header(input: &mut &[u8]) -> tinyklv::Result<(u8, Framing)> {
    Ok((decb::u8(input)?, Framing::Sliced(decb::u8_as_usize(input)?)))
}

fn header_enc(key: u8, out: &mut Vec<u8>) {
    out.push(key);
}

macro_rules! reading {
    ($name:ident, $header:path, $header_enc:path) => {
        #[derive(Klv)]
        #[klv(keylen(dec = $header, enc = $header_enc))]
        struct $name {
            #[klv(key = 0x01, dec = decb::u8, enc = *encb::u8)]
            id: u8,
        }
    };
}
reading!(Reading, header, header_enc);

fn main() {}
