//! book reference - keys no field declares, and values a decoder
//! rejects: `unknown(..)`, `#[klv(unknown)]`, `deny_unknown_keys`,
//! `strict`
use tinyklv::dec::binary as decb;
use tinyklv::enc::binary as encb;
use tinyklv::prelude::*;

#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(
        dec = decb::u8,
        enc = encb::u8,
    ),
    len(
        dec = decb::u8_as_usize,
        enc = encb::u8_from_usize,
    ),
    unknown(
        dec = dec_unknown,
        enc = &enc_unknown,
    ),
)]
/// `unknown(..)` and `#[klv(unknown)]`: keeps unknown items
struct Kept {
    #[klv(
        key = 0x01,
        dec = decb::u8,
        enc = *encb::u8,
    )]
    seq: u8,

    #[klv(
        key = 0x02,
        dec = decb::be_u16,
        enc = *encb::be_u16,
    )]
    temp: u16,

    #[klv(unknown)]
    extra: Vec<(u8, Vec<u8>)>,
}

/// Reads the rest of an unknown item: its length, then its
/// value
fn dec_unknown(_key: u8) -> impl Fn(&mut &[u8]) -> tinyklv::Result<Vec<u8>> {
    |input| {
        let len = decb::u8_as_usize(input)?;
        let Some((value, rest)) = input.split_at_checked(len) else {
            // cut short: consume the rest, so a stream waits
            input.finish();
            return Err(tinyklv::winnow::error::ContextError::from_input(input));
        };
        *input = rest;
        Ok(value.to_vec())
    }
}

/// Writes the rest of a kept item: its length, then its
/// value
fn enc_unknown(value: &[u8], out: &mut Vec<u8>) {
    encb::u8_from_usize(value.len(), out);
    out.extend_from_slice(value);
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(
        dec = decb::u8,
        enc = encb::u8,
    ),
    len(
        dec = decb::u8_as_usize,
        enc = encb::u8_from_usize,
    ),
    deny_unknown_keys,
)]
/// `deny_unknown_keys`: page 1.1's struct, no unknown key
struct Denied {
    #[klv(
        key = 0x01,
        dec = decb::u8,
        enc = *encb::u8,
    )]
    seq: u8,

    #[klv(
        key = 0x02,
        dec = decb::be_u16,
        enc = *encb::be_u16,
    )]
    temp: u16,
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(
        dec = decb::u8,
        enc = encb::u8,
    ),
    len(
        dec = decb::u8_as_usize,
        enc = encb::u8_from_usize,
    ),
    strict,
)]
/// `strict`: page 1.1's struct, every value read whole
struct Strict {
    #[klv(
        key = 0x01,
        dec = decb::u8,
        enc = *encb::u8,
    )]
    seq: u8,

    #[klv(
        key = 0x02,
        dec = decb::be_u16,
        enc = *encb::be_u16,
    )]
    temp: u16,
}

fn main() {
    let kept = Kept {
        seq: 42,
        temp: 2350,
        extra: vec![(0x07, vec![0xFF])],
    };
    #[rustfmt::skip]
    let wire = [
        0x01, 0x01, 0x2A,       // seq:  key, len, 42
        0x07, 0x01, 0xFF,       // no field: key, len, FF
        0x02, 0x02, 0x09, 0x2E, // temp: key, len, 2350
    ];

    // the kept item is written after the fields
    let mut encoded = Vec::new();
    kept.encode_value(&mut encoded);
    println!("{kept:?} -> {encoded:02X?}");
    #[rustfmt::skip]
    assert_eq!(encoded, [
        0x01, 0x01, 0x2A,       // seq:  key, len, 42
        0x02, 0x02, 0x09, 0x2E, // temp: key, len, 2350
        0x07, 0x01, 0xFF,       // kept: key, len, FF
    ]);
    let decoded = Kept::decode_value(&mut wire.as_slice());
    assert_eq!(decoded.unwrap(), kept);

    // deny_unknown_keys: key 07 is an error
    let error = Denied::decode_value(&mut wire.as_slice()).unwrap_err();
    assert_eq!(ErrorKind::of(&error), Some(ErrorKind::UnknownKey));

    // strict: temp padded to 4 bytes is an error
    #[rustfmt::skip]
    let wire = [
        0x01, 0x01, 0x2A,       // seq:  key, len, 42
        0x02, 0x04,             // temp: key, len 4
        0x09, 0x2E, 0x00, 0x00, // 2350, then 2 bytes of padding
    ];
    let error = Strict::decode_value(&mut wire.as_slice()).unwrap_err();
    assert_eq!(ErrorKind::of(&error), Some(ErrorKind::MalformedValue));
}
