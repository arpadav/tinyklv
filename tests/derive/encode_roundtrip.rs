use tinyklv::dec::binary as decb;
use tinyklv::dec::string as decs;
use tinyklv::enc::binary as encb;
use tinyklv::enc::string as encs;
use tinyklv::prelude::*;

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct AllNumerics {
    #[klv(
        key = 0x01,
        dec = decb::u8,
        enc = *encb::u8,
    )]
    u8_field: u8,

    #[klv(
        key = 0x02,
        dec = decb::be_u16,
        enc = *encb::be_u16,
    )]
    u16_field: u16,

    #[klv(
        key = 0x03,
        dec = decb::be_u32,
        enc = *encb::be_u32,
    )]
    u32_field: u32,

    #[klv(
        key = 0x04,
        dec = decb::be_u64,
        enc = *encb::be_u64,
    )]
    u64_field: u64,

    #[klv(
        key = 0x05,
        dec = decb::be_i16,
        enc = *encb::be_i16,
    )]
    i16_field: i16,

    #[klv(
        key = 0x06,
        dec = decb::be_i32,
        enc = *encb::be_i32,
    )]
    i32_field: i32,
}

#[test]
/// Tests encode/decode roundtrip across `u8/u16/u32/u64/i16/i32` fields with typical non-zero values including negatives.
fn all_numerics_roundtrip_typical() {
    let original = AllNumerics {
        u8_field: 0xAB,
        u16_field: 0x1234,
        u32_field: 0xDEAD_BEEF,
        u64_field: 0x0102_0304_0506_0708,
        i16_field: -1000,
        i32_field: -100_000,
    };
    let mut encoded = Vec::new();
    original.encode_value(&mut encoded);
    let decoded = AllNumerics::decode_value(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original);
}

#[test]
/// Tests that all-zero numeric fields roundtrip through encode/decode unchanged.
fn all_numerics_roundtrip_zeros() {
    let original = AllNumerics {
        u8_field: 0,
        u16_field: 0,
        u32_field: 0,
        u64_field: 0,
        i16_field: 0,
        i32_field: 0,
    };
    let mut encoded = Vec::new();
    original.encode_value(&mut encoded);
    let decoded = AllNumerics::decode_value(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original);
}

#[test]
/// Tests that type-maximum values for every numeric field roundtrip correctly.
fn all_numerics_roundtrip_max_values() {
    let original = AllNumerics {
        u8_field: u8::MAX,
        u16_field: u16::MAX,
        u32_field: u32::MAX,
        u64_field: u64::MAX,
        i16_field: i16::MAX,
        i32_field: i32::MAX,
    };
    let mut encoded = Vec::new();
    original.encode_value(&mut encoded);
    let decoded = AllNumerics::decode_value(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original);
}

#[test]
/// Tests that `i16::MIN` and `i32::MIN` roundtrip correctly (exercises two's-complement sign-bit boundary).
fn all_numerics_roundtrip_min_signed() {
    let original = AllNumerics {
        u8_field: 0,
        u16_field: 0,
        u32_field: 0,
        u64_field: 0,
        i16_field: i16::MIN,
        i32_field: i32::MIN,
    };
    let mut encoded = Vec::new();
    original.encode_value(&mut encoded);
    let decoded = AllNumerics::decode_value(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original);
}

// --------------------------------------------------
// struct with optional field roundtrip
// --------------------------------------------------

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct WithOptionalRoundtrip {
    #[klv(
        key = 0x01,
        dec = decb::be_u32,
        enc = *encb::be_u32,
    )]
    required: u32,
    #[klv(
        key = 0x02,
        dec = decb::be_u32,
        enc = *encb::be_u32,
    )]
    optional: Option<u32>,
}

#[test]
/// Verifies roundtrip when the optional field carries `Some(value)`.
fn optional_some_roundtrip() {
    let original = WithOptionalRoundtrip {
        required: 0xABCD,
        optional: Some(0x1234),
    };
    let mut encoded = Vec::new();
    original.encode_value(&mut encoded);
    let decoded = WithOptionalRoundtrip::decode_value(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original);
}

#[test]
/// Verifies that a `None` optional is omitted on encode and decodes back to `None`.
fn optional_none_roundtrip() {
    let original = WithOptionalRoundtrip {
        required: 0xABCD,
        optional: None,
    };
    let mut encoded = Vec::new();
    original.encode_value(&mut encoded);
    let decoded = WithOptionalRoundtrip::decode_value(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original);
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct WithStringRoundtrip {
    #[klv(
        key = 0x01,
        dec = decb::be_u16,
        enc = *encb::be_u16,
    )]
    id: u16,
    #[klv(
        key = 0x02,
        varlen = true,
        dec = decs::to_string_utf8,
        enc = &encs::from_string_utf8
    )]
    name: String,
}

#[test]
/// Tests roundtrip of a plain ASCII UTF-8 string field.
fn string_field_roundtrip_ascii() {
    let original = WithStringRoundtrip {
        id: 1,
        name: String::from("MISSION01"),
    };
    let mut encoded = Vec::new();
    original.encode_value(&mut encoded);
    let decoded = WithStringRoundtrip::decode_value(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original);
}

#[test]
/// Tests roundtrip when the UTF-8 string field is empty.
fn string_field_roundtrip_empty() {
    let original = WithStringRoundtrip {
        id: 0,
        name: String::new(),
    };
    let mut encoded = Vec::new();
    original.encode_value(&mut encoded);
    let decoded = WithStringRoundtrip::decode_value(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original);
}

#[test]
/// Tests roundtrip for a UTF-8 string containing multi-byte characters and an emoji code point.
fn string_field_roundtrip_unicode() {
    let original = WithStringRoundtrip {
        id: 42,
        name: String::from("Héllo 🌍"),
    };
    let mut encoded = Vec::new();
    original.encode_value(&mut encoded);
    let decoded = WithStringRoundtrip::decode_value(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original);
}

// --------------------------------------------------
// writer append-semantics: the buffer-writing API must APPEND to `out`, never clear or
// overwrite pre-existing bytes. this is the invariant the writer form introduces over the
// old owned-return form, and it is what lets one buffer be reused across many records
// --------------------------------------------------

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"\x52\x53",
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct FramedRecord {
    #[klv(key = 0x01, dec = decb::be_u16, enc = *encb::be_u16)]
    a: u16,
    #[klv(key = 0x02, dec = decb::be_u32, enc = *encb::be_u32)]
    b: u32,
}

#[test]
/// `encode_value` appends to a non-empty buffer: the pre-existing prefix is left intact and
/// the encoded body follows it byte-for-byte (identical to encoding into a fresh buffer).
fn encode_value_appends_to_nonempty_buffer() {
    let rec = AllNumerics {
        u8_field: 0xAB,
        u16_field: 0x1234,
        u32_field: 0xDEAD_BEEF,
        u64_field: 0x0102_0304_0506_0708,
        i16_field: -1000,
        i32_field: -100_000,
    };
    let prefix: &[u8] = b"\xDE\xAD\xBE\xEF";
    let mut fresh = Vec::new();
    rec.encode_value(&mut fresh);

    let mut prefilled = prefix.to_vec();
    rec.encode_value(&mut prefilled);

    assert!(prefilled.starts_with(prefix), "prefix was clobbered");
    assert_eq!(
        prefilled.strip_prefix(prefix),
        Some(fresh.as_slice()),
        "appended body differs from a fresh-buffer encode",
    );
}

#[test]
/// `encode_frame` appends to a non-empty buffer (same invariant as `encode_value`), and the
/// frame written into the fresh buffer still decodes back to the original record.
fn encode_frame_appends_to_nonempty_buffer() {
    let rec = FramedRecord {
        a: 0x1234,
        b: 0xDEAD_BEEF,
    };
    let prefix: &[u8] = b"\x99\x88";
    let mut fresh = Vec::new();
    rec.encode_frame(&mut fresh);

    let mut prefilled = prefix.to_vec();
    rec.encode_frame(&mut prefilled);

    assert!(prefilled.starts_with(prefix), "prefix was clobbered");
    assert_eq!(prefilled.strip_prefix(prefix), Some(fresh.as_slice()));

    let decoded = FramedRecord::decode_frame(&mut fresh.as_slice()).unwrap();
    assert_eq!(decoded, rec);
}

#[test]
/// Leaf and BER encoders append into an existing buffer rather than replacing its contents.
fn leaf_and_ber_encoders_append() {
    let mut buf = b"\x01\x02".to_vec();
    encb::be_u32(0xCAFE_BABE, &mut buf);
    tinyklv::codecs::ber::enc::ber_length(200_u64, &mut buf);
    assert_eq!(
        buf,
        // existing prefix, then be_u32, then BER long-form length (200 >= 128)
        vec![0x01, 0x02, 0xCA, 0xFE, 0xBA, 0xBE, 0x80 | 1, 200],
    );
}

#[test]
#[should_panic(expected = "exceeds the 1-byte KLV length prefix")]
/// A variable-width field whose encoded body is longer than the (1-byte) length prefix can
/// represent must abort loudly: a silently-wrapped length (`300 as u8 == 44`) would emit a
/// mis-framed, undecodable packet. Guards the back-patch path against length-prefix overflow.
fn oversized_field_aborts_rather_than_corrupting() {
    let original = WithStringRoundtrip {
        id: 7,
        name: "x".repeat(300),
    };
    let mut encoded = Vec::new();
    original.encode_value(&mut encoded);
}

#[test]
/// The one-shot direct `decode_value` rewinds a truncated trailing triple, leaving the cursor
/// exactly where the streaming path's `reset` would. Without that rewind, an embedded
/// `Vec<T>::decode_value` (which reads the residual cursor to continue) would observe the trailing
/// byte already consumed. Decodes the complete fields, then asserts the lone trailing key byte
/// remains unconsumed.
fn decode_value_rewinds_truncated_trailing_triple() {
    let original = AllNumerics {
        u8_field: 0xAB,
        u16_field: 0x1234,
        u32_field: 0xDEAD_BEEF,
        u64_field: 0x0102_0304_0506_0708,
        i16_field: -1000,
        i32_field: -100_000,
    };
    let mut encoded = Vec::new();
    original.encode_value(&mut encoded);
    // append an incomplete trailing triple: a key byte with no length/value after it
    encoded.push(0xFE);

    let mut input: &[u8] = &encoded;
    let decoded = AllNumerics::decode_value(&mut input).unwrap();
    assert_eq!(decoded, original);
    // the incomplete triple was rewound, so its lone key byte is left for a surrounding parser
    assert_eq!(
        input,
        &[0xFE],
        "truncated trailing triple should be rewound, not consumed"
    );
}

#[test]
/// Companion to the trailing-triple test, covering the *other* one-shot rewind site: a complete
/// key+len whose declared value body overruns the remaining bytes. The decoder must rewind the
/// whole `key,len,partial-value` triple (not just up to the value) so a surrounding parser sees the
/// triple intact. Uses an optional trailing field so `finalize` still succeeds on the required one.
fn decode_value_rewinds_truncated_value_body() {
    // required field present in full: key 0x01, len 4, be_u32 0x0000_ABCD
    let mut encoded = vec![0x01, 0x04, 0x00, 0x00, 0xAB, 0xCD];
    let prefix_len = encoded.len();
    // optional field 0x02 declares a 4-byte value but only 2 bytes follow (truncated mid-value)
    let truncated_triple = [0x02_u8, 0x04, 0xDE, 0xAD];
    encoded.extend_from_slice(&truncated_triple);

    let mut input: &[u8] = &encoded;
    let decoded = WithOptionalRoundtrip::decode_value(&mut input).unwrap();
    // the complete required field decoded; the truncated optional stayed `None`
    assert_eq!(
        decoded,
        WithOptionalRoundtrip {
            required: 0xABCD,
            optional: None,
        }
    );
    // the entire truncated triple (key+len+partial value) was rewound, not partially consumed
    assert_eq!(
        input,
        &encoded[prefix_len..],
        "truncated value body should rewind the whole triple"
    );
}
