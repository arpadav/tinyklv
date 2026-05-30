//! Tests for the opt-in `#[klv(trait_fallback)]` container flag.
//!
//! When set, any field lacking an explicit `enc`/`dec` and not matched by a
//! container `default(..)` falls back to the `EncodeValue`/`DecodeValue` trait
//! implementations for the field's type. Without the flag, the existing
//! `UnimplementedEncode`/`UnimplementedDecode` errors still fire
use tinyklv::Klv;
use tinyklv::dec::binary as decb;
use tinyklv::enc::binary as encb;
use tinyklv::prelude::*;

// --------------------------------------------------
// hand-impl leaf struct
// --------------------------------------------------
#[derive(Debug, PartialEq, Clone)]
struct Id(u16);
impl tinyklv::DecodeValue<&[u8]> for Id {
    fn decode_value(input: &mut &[u8]) -> tinyklv::Result<Self> {
        Ok(Id(decb::be_u16(input)?))
    }
}
impl tinyklv::EncodeValue for Id {
    fn encode_value(&self, out: &mut Vec<u8>) {
        encb::be_u16(self.0, out);
    }
}

// --------------------------------------------------
// hand-impl leaf enum
// --------------------------------------------------
#[derive(Debug, PartialEq, Clone)]
enum Mode {
    A,
    B,
}
impl tinyklv::DecodeValue<&[u8]> for Mode {
    fn decode_value(input: &mut &[u8]) -> tinyklv::Result<Self> {
        let b = decb::u8(input)?;
        match b {
            0 => Ok(Mode::A),
            1 => Ok(Mode::B),
            _ => Err(::tinyklv::__export::winnow::error::ContextError::new()),
        }
    }
}
impl tinyklv::EncodeValue for Mode {
    fn encode_value(&self, out: &mut Vec<u8>) {
        match self {
            Mode::A => out.push(0),
            Mode::B => out.push(1),
        }
    }
}

// --------------------------------------------------
// nested derived struct
// --------------------------------------------------
#[derive(Klv, Debug, PartialEq, Clone)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
    trait_fallback,
)]
struct Inner {
    #[klv(
        key = 0x11,
        dec = decb::be_u16,
        enc = *encb::be_u16,
    )]
    value: u16,
}

// --------------------------------------------------
// case 1, 2, 3, 4: leaf struct, leaf enum, Option, nested derived
// --------------------------------------------------
#[derive(Klv, Debug, PartialEq, Clone)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
    trait_fallback,
)]
struct OuterFallback {
    #[klv(key = 0x01)]
    id: Id,

    #[klv(key = 0x02)]
    mode: Mode,

    #[klv(key = 0x03)]
    opt_id: Option<Id>,

    #[klv(key = 0x04)]
    inner: Inner,
}

#[test]
/// Leaf `Id(u16)` with hand-written trait impls - no field xcoder, no default
fn fallback_leaf_struct_roundtrip() {
    let original = OuterFallback {
        id: Id(0xBEEF),
        mode: Mode::A,
        opt_id: Some(Id(42)),
        inner: Inner { value: 0x1234 },
    };
    let mut encoded = Vec::new();
    original.encode_value(&mut encoded);
    let decoded = OuterFallback::decode_value(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original);
}

#[test]
/// Leaf enum `Mode` with hand-written trait impls - roundtrip both variants
fn fallback_enum_roundtrip() {
    for mode in [Mode::A, Mode::B] {
        let original = OuterFallback {
            id: Id(1),
            mode: mode.clone(),
            opt_id: None,
            inner: Inner { value: 0 },
        };
        let mut encoded = Vec::new();
        original.encode_value(&mut encoded);
        let decoded = OuterFallback::decode_value(&mut encoded.as_slice()).unwrap();
        assert_eq!(decoded.mode, mode);
    }
}

#[test]
/// `Option<Id>` field absent from input resolves to `None`
fn fallback_option_none() {
    let original = OuterFallback {
        id: Id(7),
        mode: Mode::B,
        opt_id: None,
        inner: Inner { value: 0xAA },
    };
    let mut encoded = Vec::new();
    // id
    encoded.push(0x01);
    encoded.push(2);
    Id(7).encode_value(&mut encoded);
    // mode
    encoded.push(0x02);
    encoded.push(1);
    Mode::B.encode_value(&mut encoded);
    // skip opt_id
    // inner
    let mut inner_bytes = Vec::new();
    (Inner { value: 0xAA }).encode_value(&mut inner_bytes);
    encoded.push(0x04);
    encoded.push(inner_bytes.len() as u8);
    encoded.extend(inner_bytes);

    let decoded = OuterFallback::decode_value(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original);
}

#[test]
/// Nested derived struct with only `key` - trait fallback via derive-generated impl
fn fallback_nested_derived_roundtrip() {
    let original = OuterFallback {
        id: Id(u16::MAX),
        mode: Mode::A,
        opt_id: Some(Id(u16::MIN)),
        inner: Inner { value: 0xCAFE },
    };
    let mut encoded = Vec::new();
    original.encode_value(&mut encoded);
    let decoded = OuterFallback::decode_value(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded.inner.value, 0xCAFE);
    assert_eq!(decoded, original);
}

// --------------------------------------------------
// case 5: explicit field xcoder beats fallback
// --------------------------------------------------
fn custom_id_dec(input: &mut &[u8]) -> tinyklv::Result<Id> {
    let raw = decb::be_u16(input)?;
    Ok(Id(raw ^ 0xFFFF))
}
fn custom_id_enc(id: &Id, out: &mut Vec<u8>) {
    encb::be_u16(id.0 ^ 0xFFFF, out);
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
    trait_fallback,
)]
struct PrecedenceField {
    #[klv(
        key = 0x01,
        dec = custom_id_dec,
        enc = custom_id_enc,
    )]
    custom: Id,
    #[klv(key = 0x02)]
    fallback: Id,
}

#[test]
/// Explicit field-level xcoder takes precedence over `trait_fallback`
fn fallback_precedence_field_xcoder_beats_fallback() {
    let original = PrecedenceField {
        custom: Id(0x1234),
        fallback: Id(0x5678),
    };
    let mut encoded = Vec::new();
    original.encode_value(&mut encoded);
    let decoded = PrecedenceField::decode_value(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original);
    // also confirm the custom path was taken for `custom` by inspecting the
    // encoded bytes: the custom encoder xors with 0xFFFF
    let custom_key_pos = encoded.iter().position(|&b| b == 0x01).unwrap();
    let custom_bytes = &encoded[custom_key_pos + 2..custom_key_pos + 4];
    assert_eq!(custom_bytes, &[0xED, 0xCB]); // 0x1234 ^ 0xFFFF = 0xEDCB
}

// --------------------------------------------------
// case 6: container `default(..)` beats fallback
// --------------------------------------------------
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
    default(typ = Id, dec = custom_id_dec, enc = custom_id_enc),
    trait_fallback,
)]
struct PrecedenceDefault {
    #[klv(key = 0x01)]
    via_default: Id,
}

#[test]
/// Container-level `default(..)` takes precedence over `trait_fallback`
fn fallback_precedence_default_beats_fallback() {
    let original = PrecedenceDefault {
        via_default: Id(0xABCD),
    };
    let mut encoded = Vec::new();
    original.encode_value(&mut encoded);
    // container default uses custom_id_enc which xors
    let key_pos = encoded.iter().position(|&b| b == 0x01).unwrap();
    assert_eq!(&encoded[key_pos + 2..key_pos + 4], &[0x54, 0x32]); // 0xABCD ^ 0xFFFF
    let decoded = PrecedenceDefault::decode_value(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original);
}
