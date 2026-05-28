//! `Vec<T>::decode_value` blanket implementation tests
//!
//! Verifies the semantics of decoding a contiguous byte stream into a
//! `Vec<T>` where `T: DecodeValue`.  Key behaviours documented here: empty
//! stream yields empty `Vec`, unframed concatenated values collapse via
//! last-wins merge inside a single `decode_value` call, `deny_unknown_keys`
//! stops the blanket on the first unknown key, and the cursor is left at
//! the correct position on clean EOF.  Contrasts with `Decoder<T>` for
//! fragmented streaming use cases
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use tinyklv::dec::binary as decb;
use tinyklv::enc::binary as encb;
use tinyklv::prelude::*;

/// Two-field KLV packet used to test `Vec<T>::decode_value` merge semantics
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct Reading {
    #[klv(
        key = 0x01,
        dec = decb::be_u16,
        enc = *encb::be_u16,
    )]
    sensor_id: u16,

    #[klv(
        key = 0x02,
        dec = decb::be_i32,
        enc = *encb::be_i32,
    )]
    value: i32,
}

#[test]
/// `Vec<T>::decode_value` on empty stream returns empty Vec
fn vec_decode_value_empty_stream() {
    let result = Vec::<Reading>::decode_value(&mut &[][..]).unwrap();
    assert!(result.is_empty());
}

#[test]
/// `Vec<T>::decode_value` decodes a single encoded value
fn vec_decode_value_single_item() {
    let r = Reading {
        sensor_id: 1,
        value: 2350,
    };
    let encoded = r.encode_value();
    let result = Vec::<Reading>::decode_value(&mut encoded.as_slice()).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0], r);
}

#[test]
/// Without sentinel framing, `decode_value` for the inner `T` consumes all
/// KLV triples in one pass. Duplicate keys use last-wins semantics, so
/// concatenating two unframed values produces a single merged element
fn vec_decode_value_unframed_merge_last_wins() {
    let r1 = Reading {
        sensor_id: 1,
        value: 100,
    };
    let r2 = Reading {
        sensor_id: 2,
        value: 200,
    };
    let mut stream = r1.encode_value();
    stream.extend(r2.encode_value());

    let result = Vec::<Reading>::decode_value(&mut stream.as_slice()).unwrap();
    assert_eq!(
        result.len(),
        1,
        "unframed: one decode_value call consumes all triples"
    );
    assert_eq!(result[0].sensor_id, 2);
    assert_eq!(result[0].value, 200);
}

/// Single-key KLV packet used to demonstrate last-wins collapse in `Vec<T>`
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct SingleField {
    #[klv(
        key = 0x01,
        dec = decb::be_u32,
        enc = *encb::be_u32,
    )]
    val: u32,
}

#[test]
/// With a single-key struct, each `encode_value` produces identical key
/// triples. Concatenated without framing, `decode_value` reads them all
/// in one pass (last-wins), so `Vec<T>` collects exactly one element
fn vec_decode_value_single_key_last_wins() {
    let items: Vec<u32> = vec![10, 20, 30, 40, 50];
    let stream: Vec<u8> = items
        .iter()
        .flat_map(|&v| SingleField { val: v }.encode_value())
        .collect();

    let result = Vec::<SingleField>::decode_value(&mut stream.as_slice()).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].val, 50);
}

/// Three-key KLV packet with `deny_unknown_keys` to exercise blanket-stop behaviour on unknown keys
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
    deny_unknown_keys,
)]
struct MultiKey {
    #[klv(
        key = 0x01,
        dec = decb::u8,
        enc = *encb::u8,
    )]
    a: u8,

    #[klv(
        key = 0x02,
        dec = decb::u8,
        enc = *encb::u8,
    )]
    b: u8,

    #[klv(
        key = 0x03,
        dec = decb::u8,
        enc = *encb::u8,
    )]
    c: u8,
}

#[test]
/// Roundtrip: encode one `MultiKey` value, decode via `Vec<T>::decode_value`,
/// verify the single element matches
fn vec_decode_value_roundtrip_single() {
    let mk = MultiKey {
        a: 10,
        b: 20,
        c: 30,
    };
    let encoded = mk.encode_value();
    let result = Vec::<MultiKey>::decode_value(&mut encoded.as_slice()).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0], mk);
}

#[test]
/// Two distinct `MultiKey` values decoded via `Vec<T>::decode_value`
/// Without sentinel framing both are consumed in one `decode_value`
/// call (last-wins merge), so the Vec has one element with p2 values
fn vec_decode_value_two_multikey_merge() {
    let p1 = MultiKey { a: 1, b: 2, c: 3 };
    let p2 = MultiKey {
        a: 10,
        b: 20,
        c: 30,
    };
    let mut stream = p1.encode_value();
    stream.extend(p2.encode_value());

    let result = Vec::<MultiKey>::decode_value(&mut stream.as_slice()).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0], p2);
}

#[test]
/// Trailing junk after a valid `MultiKey`: the unknown key `0xFF` claims
/// length `0x10` (16 bytes) but none remain
///
/// `MultiKey` has `deny_unknown_keys` set, so the unknown key itself is
/// what fails the inner `decode_value` (via the pre-take gate) - not the
/// short read. The inner call returns `Err` after consuming the valid
/// three fields, so the `Vec<T>` blanket stops with an empty accumulator
///
/// This documents the boundary of the `Vec<T>` blanket: it is appropriate
/// for "repeated inner field inside a parent packet" semantics, NOT for
/// "stream of top-level packets across fragmented reads." For that use
/// case, see `Decoder<T>` / the streaming tests
fn vec_decode_value_trailing_junk_yields_empty() {
    let mk = MultiKey { a: 1, b: 2, c: 3 };
    let mut stream = mk.encode_value();
    // --------------------------------------------------
    // unknown key=0xFF, len=0x10 (16 bytes that don't exist)
    // --------------------------------------------------
    stream.extend_from_slice(&[0xFF, 0x10]);

    let result = Vec::<MultiKey>::decode_value(&mut stream.as_slice()).unwrap();
    assert!(
        result.is_empty(),
        "Vec<T> blanket stops on inner Err; use Decoder<T> for streaming",
    );
}

#[test]
/// Companion to the trailing-junk test: verifies the cursor-hazard fix
/// in the `Vec<T>` blanket. After the blanket exits, the outer cursor
/// must NOT be advanced past bytes that a surrounding parser may still
/// want to consume
///
/// Here we hand the blanket two valid `MultiKey` packets back-to-back
/// with no junk. It consumes them both (last-wins merge inside one
/// `decode_value` call, so `len == 1`), then on the next iteration hits
/// clean EOF, rewinds to that checkpoint, and returns. The outer slice
/// must be empty at the end
fn vec_decode_value_cursor_safe_on_clean_eof() {
    let p1 = MultiKey { a: 1, b: 2, c: 3 };
    let p2 = MultiKey {
        a: 10,
        b: 20,
        c: 30,
    };
    let mut stream = p1.encode_value();
    stream.extend(p2.encode_value());
    let mut cursor: &[u8] = stream.as_slice();

    let _ = Vec::<MultiKey>::decode_value(&mut cursor).unwrap();
    assert!(
        cursor.is_empty(),
        "Vec<T> blanket must drain the outer cursor on clean EOF",
    );
}
