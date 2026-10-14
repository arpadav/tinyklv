//! Streaming interaction with `Vec<T>` inside a parent packet
//!
//! Covers the legitimate "repeated inner field" use of the
//! `impl<S, T> DecodeValue<S> for Vec<T>` blanket: a parent `#[derive(Klv)]`
//! struct that carries a `Vec<Reading>` as one of its fields, meaning
//! the `0x10` tag may appear N times inside one parent packet
//!
//! Then wraps the same parent in a `Decoder<Parent>` and streams
//! several complete parent packets back-to-back to make sure the
//! cross-packet streaming path also works when nested accumulation is
//! in play
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::beacon::Beacon;
use tinyklv::dec::binary as decb;
use tinyklv::enc::binary as encb;
use tinyklv::prelude::*;
use tinyklv::traits::PartialError;

/// A 2-byte sensor reading: a `channel` triple followed by a `value` triple
#[derive(Klv, Debug, Clone, Copy, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct Reading {
    #[klv(key = 0x01, dec = decb::u8, enc = *encb::u8)]
    channel: u8,
    #[klv(key = 0x02, dec = decb::u8, enc = *encb::u8)]
    value: u8,
}

/// A sentinel-framed packet holding an id plus a repeated `Reading` field
///
/// - `sentinel = b"PAR"` gives the `Decoder<Parent>` streaming API a way
///   to locate packet boundaries inside a byte stream
/// - `trait_fallback` routes the `Vec<Reading>` field through the blanket
///   `DecodeValue` impl for `Vec<T>` (and `EncodeValue`) without an explicit
///   field-level xcoder
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"PAR",
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
    trait_fallback,
    allow_unimplemented_encode,
)]
struct Parent {
    #[klv(key = 0x0F, dec = decb::u8)]
    id: u8,
    #[klv(key = 0x10)]
    readings: Vec<Reading>,
}

/// Hand-frames one `Parent` packet carrying `readings` as its repeated `0x10` field
///
/// The wire is the sentinel `PAR`, the body length, then the body: the `id` triple and one `0x10`
/// triple whose value is every reading's own encoding, back-to-back
///
/// # Arguments
///
/// * `id` - The value of `Parent::id`
/// * `readings` - The readings the `0x10` value carries, in order
///
/// # Returns
///
/// The packet's bytes
fn parent_bytes(id: u8, readings: &[Reading]) -> Vec<u8> {
    // --------------------------------------------------
    // hand-frame the packet: sentinel + packet_len + body
    // body = id triple + one 0x10 triple whose sub-body is the
    // concatenation of `readings.encode_value()`
    // --------------------------------------------------
    let mut inner = Vec::new();
    for r in readings {
        r.encode_value(&mut inner);
    }
    let mut body = vec![0x0F, 0x01, id];
    body.push(0x10);
    body.push(inner.len() as u8);
    body.extend(inner);
    let mut out = Vec::new();
    out.extend_from_slice(b"PAR");
    out.push(body.len() as u8);
    out.extend(body);
    out
}

#[test]
/// Single parent packet carrying three repeated readings decodes via
/// `decode_frame` (sentinel-aware) into a `Parent`
///
/// The inner `Vec<Reading>` blanket, running on an unframed sub-slice,
/// last-wins merges the three readings into a single `Reading` element. This is the documented
/// behavior for the Vec<T>-inside-a-parent use case without per-reading framing; see
/// `vec_decode_value_unframed_merge_last_wins` for the top-level
/// counterpart
fn parent_with_repeated_readings_decodes() {
    let readings = [
        Reading {
            channel: 1,
            value: 10,
        },
        Reading {
            channel: 2,
            value: 20,
        },
        Reading {
            channel: 3,
            value: 30,
        },
    ];
    let bytes = parent_bytes(7, &readings);

    let parent = Parent::decode_frame(&mut bytes.as_slice()).unwrap();
    assert_eq!(parent.id, 7);
    assert_eq!(parent.readings.len(), 1);
    assert_eq!(parent.readings[0].channel, 3);
    assert_eq!(parent.readings[0].value, 30);
}

#[test]
/// Junk bytes between sentinel-framed parent packets are skipped. All three parents decode in order
/// with correct inner readings
fn decoder_parent_with_junk_between() {
    let a = parent_bytes(
        1,
        &[Reading {
            channel: 0x11,
            value: 0x22,
        }],
    );
    let b = parent_bytes(
        2,
        &[Reading {
            channel: 0x33,
            value: 0x44,
        }],
    );
    let c = parent_bytes(
        3,
        &[Reading {
            channel: 0x55,
            value: 0x66,
        }],
    );
    let mut blob = Vec::new();
    // junk before first sentinel
    blob.extend_from_slice(&[0x00, 0x00, 0xFF, 0xDE, 0xAD]);
    blob.extend(a);
    // junk between first and second
    blob.extend_from_slice(&[0xFF, 0x00, 0xBE, 0xEF]);
    blob.extend(b);
    // junk between second and third
    blob.extend_from_slice(&[0x00, 0xCA, 0xFE]);
    blob.extend(c);

    let mut dec = Parent::decoder();
    dec.feed(&blob);
    let mut ids = Vec::new();
    for r in dec.iter() {
        ids.push(r.id);
    }
    assert_eq!(ids, vec![1, 2, 3]);
}

#[test]
/// `Vec<T>::decode_partial` never pushes an element that consumed zero bytes:
/// an all-optional `T` on empty input is `Ready` with an empty `Vec`, not one
/// all-`None` element
fn vec_decode_partial_all_optional_empty_stream() {
    match Vec::<Beacon>::decode_partial(&mut &[][..]) {
        Ok(Packet::Ready(decoded)) => assert_eq!(decoded, []),
        other => panic!("expected Ready, got {other:?}"),
    }
}

#[test]
/// `Vec<T>::decode_partial` finalizes and pushes a cut final element that consumed
/// bytes: a `heading` frame lands, then the `altitude` frame is cut
fn vec_decode_partial_finalizes_cut_last_element() {
    #[rustfmt::skip]
    let wire = [
        // heading = 5
        0x02, 0x01, 0x05,
        // altitude, one of its two bytes
        0x01, 0x02, 0x00,
    ];
    let mut cursor = &wire[..];
    match Vec::<Beacon>::decode_partial(&mut cursor) {
        Ok(Packet::Ready(decoded)) => assert_eq!(
            decoded,
            [Beacon {
                altitude: None,
                heading: Some(5),
            }]
        ),
        other => panic!("expected Ready, got {other:?}"),
    }
    assert_eq!(
        cursor,
        &wire[3..],
        "the cut altitude frame stays unconsumed"
    );
}

#[test]
/// `Vec<T>::decode_partial` does not push a cut element that cannot finalize: the cursor
/// rewinds to the element's start, so the call after the rest arrives reads it whole
fn vec_decode_partial_rewinds_cut_unfinalized_element() {
    #[rustfmt::skip]
    let wire = [
        // channel = 3
        0x01, 0x01, 0x03,
        // value, its key and length only
        0x02, 0x01,
    ];
    let mut cursor = &wire[..];
    match Vec::<Reading>::decode_partial(&mut cursor) {
        Ok(Packet::Ready(decoded)) => assert_eq!(decoded, []),
        other => panic!("expected Ready, got {other:?}"),
    }
    assert_eq!(cursor, &wire[..], "the cut reading stays unconsumed");
}

/// A `Beacon` with `deny_unknown_keys`, so an unrecognised key is an error
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
    deny_unknown_keys,
)]
struct StrictBeacon {
    #[klv(key = 0x01, dec = decb::be_u16, enc = *encb::be_u16)]
    altitude: Option<u16>,
    #[klv(key = 0x02, dec = decb::u8, enc = *encb::u8)]
    heading: Option<u8>,
}

#[test]
/// `Vec<T>::decode_partial` propagates an element's error once that element consumed
/// bytes: a `heading` frame lands, then an unknown key is rejected
fn vec_decode_partial_propagates_error_after_consuming() {
    #[rustfmt::skip]
    let wire = [
        // heading = 5
        0x02, 0x01, 0x05,
        // unknown key 0x09
        0x09, 0x01, 0x00,
    ];
    match Vec::<StrictBeacon>::decode_partial(&mut &wire[..]).map_err(PartialError::label) {
        Err(label) if label.starts_with("key (expected") && label.contains("StrictBeacon") => {}
        other => panic!("expected an unknown-key Err naming `StrictBeacon`, got {other:?}"),
    }
}
