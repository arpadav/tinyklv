//! `deny_unknown_keys` pre-match tests.
//!
//! Verifies the codegen ordering fix: an unknown key followed by a
//! truncated value must surface an "unknown key" error BEFORE the take
//! of the declared length is attempted. The legacy ordering reported
//! "truncated" in this scenario and wasted the take.

use tinyklv::dec::binary as decb;
use tinyklv::enc::binary as encb;
use tinyklv::prelude::*;

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
    deny_unknown_keys,
)]
struct Strict {
    #[klv(key = 0x01, dec = decb::u8, enc = *encb::u8)]
    a: u8,
    #[klv(key = 0x02, dec = decb::u8, enc = *encb::u8)]
    b: u8,
}

#[test]
/// Unknown key `0xFF` claims a 16-byte value that cannot possibly fit in
/// the two remaining trailing bytes. The pre-match gate must catch the
/// unknown key BEFORE the short-read is noticed. So the error message
/// must mention "invalid key", not "truncated".
fn unknown_key_before_take_reports_unknown_not_truncated() {
    let mut stream: Vec<u8> = Vec::new();
    stream.extend_from_slice(&[0x01, 0x01, 0xAA]);
    stream.extend_from_slice(&[0x02, 0x01, 0xBB]);
    // --------------------------------------------------
    // unknown key, overlong declared length, short input
    // --------------------------------------------------
    stream.extend_from_slice(&[0xFF, 0x10]);

    let err = Strict::decode_value(&mut stream.as_slice()).unwrap_err();
    let rendered = format!("{err}");
    assert!(
        rendered.contains("invalid key"),
        "expected 'invalid key' in error, got: {rendered}",
    );
    assert!(
        !rendered.contains("truncated"),
        "unknown-key must beat truncated in error priority, got: {rendered}",
    );
}

#[test]
/// Happy path: a fully-valid packet with only known keys must decode
/// cleanly even when `deny_unknown_keys` is set.
fn strict_accepts_known_keys_only() {
    let v = Strict { a: 7, b: 42 };
    let encoded = v.encode_value();
    let decoded = Strict::decode_value(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, v);
}

#[test]
/// `DecodePartial` companion: an unknown key mid-stream surfaces as
/// the unrecoverable path - `Err(label)` carrying the "invalid key"
/// label, even if the trailing bytes would otherwise suggest
/// truncation. Codegen emits the `deny_unknown_keys` gate as a
/// `Result::Err` directly (no finalise attempt), so the label flows
/// outward unchanged.
fn unknown_key_under_decode_partial_is_err_label() {
    let mut stream: Vec<u8> = Vec::new();
    stream.extend_from_slice(&[0x01, 0x01, 0xAA]);
    stream.extend_from_slice(&[0xFF, 0x10]);

    let mut cursor: &[u8] = stream.as_slice();
    match Strict::decode_partial(&mut cursor) {
        Err(label) => assert!(
            label.contains("invalid key"),
            "label must mention 'invalid key'; got: {label}"
        ),
        other => panic!("expected Err(label), got {}", other_discriminant(&other)),
    }
}

fn other_discriminant<T, P>(p: &Result<Packet<T, P>, &'static str>) -> &'static str
where
    P: tinyklv::Partial<Final = T>,
{
    match p {
        Ok(Packet::Ready(_)) => "Ok(Ready)",
        Ok(Packet::NeedMore(_)) => "Ok(NeedMore)",
        Err(_) => "Err(label)",
    }
}
