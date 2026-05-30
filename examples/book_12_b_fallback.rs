#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(clippy::unwrap_used)]
//! See: `book/tutorial/12-default-fallback.md` for full example
use tinyklv::prelude::*;            // Klv proc-macro + traits
use tinyklv::dec::binary as decb;   // binary decoders
use tinyklv::enc::binary as encb;   // binary encoders

#[derive(Debug, PartialEq)]
struct DeviceId(u16);
impl DecodeValue<&[u8]> for DeviceId {
    fn decode_value(input: &mut &[u8]) -> tinyklv::Result<Self> {
        Ok(DeviceId(decb::be_u16(input)?))
    }
}
impl EncodeValue for DeviceId {
    fn encode_value(&self, out: &mut Vec<u8>) {
        encb::be_u16(self.0, out);
    }
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8,          enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
    trait_fallback,
)]
/// Fields lacking `enc` / `dec` fall back to the `EncodeValue` /
/// `DecodeValue` impls on the field type
struct Reading {
    #[klv(key = 0x01)]
    id: DeviceId,

    #[klv(key = 0x02)]
    opt_id: Option<DeviceId>,
}

fn main() {
    // full round-trip through the derived code paths
    let full = Reading {
        id:     DeviceId(0xBEEF),
        opt_id: Some(DeviceId(42)),
    };
    let mut full_bytes = Vec::new();
    full.encode_value(&mut full_bytes);
    let full_decoded = Reading::decode_value(
        &mut full_bytes.as_slice(),
    ).unwrap();
    assert_eq!(full_decoded, full);

    // manually constructed stream: only key 0x01 present
    let partial_bytes: &[u8] = &[
        0x01, 0x02, 0xBE, 0xEF,
    ];
    let decoded = Reading::decode_value(
        &mut &partial_bytes[..],
    ).unwrap();
    assert_eq!(decoded.id, DeviceId(0xBEEF));
    assert_eq!(decoded.opt_id, None);
}
