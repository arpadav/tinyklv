//! Streaming interaction with `Vec<T>` inside a parent packet.
//!
//! Covers the legitimate "repeated inner field" use of the
//! `impl<S, T> DecodeValue<S> for Vec<T>` blanket: a parent `#[derive(Klv)]`
//! struct that carries a `Vec<Reading>` as one of its fields, meaning
//! the `0x10` tag may appear N times inside one parent packet.
//!
//! Then wraps the same parent in a `Decoder<Parent>` and streams
//! several complete parent packets back-to-back to make sure the
//! cross-packet streaming path also works when nested accumulation is
//! in play.

use tinyklv::dec::binary as decb;
use tinyklv::enc::binary as encb;
use tinyklv::prelude::*;

// --------------------------------------------------
// inner item: a 2-byte sensor reading
// --------------------------------------------------
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

// --------------------------------------------------
// parent: sentinel-framed packet holding an id plus a repeated inner
// field.
//
// - `sentinel = b"PAR"` gives the `Decoder<Parent>` streaming API a way
//   to locate packet boundaries inside a byte stream.
// - `fallback_impls` routes the `Vec<Reading>` field through the
//   blanket `DecodeValue`/`EncodeValue` impls at `src/traits/dec.rs`
//   without needing an explicit field-level xcoder.
// --------------------------------------------------
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"PAR",
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
    fallback_impls,
    allow_unimplemented_encode,
)]
struct Parent {
    #[klv(key = 0x0F, dec = decb::u8)]
    id: u8,
    #[klv(key = 0x10)]
    readings: Vec<Reading>,
}

fn parent_bytes(id: u8, readings: &[Reading]) -> Vec<u8> {
    // --------------------------------------------------
    // hand-frame the packet: sentinel + packet_len + body.
    // body = id triple + one 0x10 triple whose sub-body is the
    // concatenation of `readings.encode_value()`.
    // --------------------------------------------------
    let mut inner = Vec::new();
    for r in readings {
        inner.extend(r.encode_value());
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
/// `decode_frame` (sentinel-aware) into a `Parent`.
///
/// The inner `Vec<Reading>` blanket, running on an unframed sub-slice,
/// last-wins merges the three readings into a single `Reading` element.
/// This is the documented behavior for the Vec<T>-inside-a-parent use
/// case without per-reading framing; see
/// `vec_decode_value_unframed_merge_last_wins` for the top-level
/// counterpart.
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
/// Stream three complete parent packets through `Decoder<Parent>` fed one
/// byte at a time. Every parent must emerge in order with no loss.
fn decoder_parent_stream_byte_by_byte() {
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
    blob.extend(a);
    blob.extend(b);
    blob.extend(c);

    let mut dec = tinyklv::Decoder::<Parent>::new();
    let mut ids = Vec::new();
    for &byte in &blob {
        dec.feed(&[byte]);
        while let Some(r) = dec.next() {
            ids.push(r.expect("Ok").id);
        }
    }
    assert_eq!(ids, vec![1, 2, 3]);
    assert!(dec.buffered().is_empty());
}
