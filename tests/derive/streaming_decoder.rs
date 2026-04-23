//! End-to-end streaming tests for [`tinyklv::Decoder`].
//!
//! Feeds a byte stream through `Decoder<T>::feed` + `next()` in several
//! granularities (one packet at a time, one byte at a time, random chunks)
//! and asserts every packet emerges with no lost progress.

use tinyklv::dec::binary as decb;
use tinyklv::enc::binary as encb;
use tinyklv::prelude::*;

// --------------------------------------------------
// fixture: a small KLV struct carrying three fixed-length fields.
//
// sentinel framing is required by `Decoder<T>`: each emitted packet on
// the wire is `sentinel + packet_len + body`, and the decoder uses the
// sentinel to find each packet's boundaries inside the streamed buffer.
// the sentinel bytes here are `b"SMPL"` purely for readability in hex
// dumps; any unique byte pattern works.
// --------------------------------------------------
#[derive(Klv, Debug, Clone, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"SMPL",
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct Sample {
    #[klv(key = 0x01, dec = decb::u8, enc = *encb::u8)]
    a: u8,
    #[klv(key = 0x02, dec = decb::u8, enc = *encb::u8)]
    b: u8,
    #[klv(key = 0x03, dec = decb::u8, enc = *encb::u8)]
    c: u8,
}

fn encoded(a: u8, b: u8, c: u8) -> Vec<u8> {
    // framed form: sentinel + len + body
    Sample { a, b, c }.encode_frame()
}

#[test]
/// One complete packet fed in one shot must yield exactly one `Ready`
/// from `next()`, then `None` (buffer empty).
fn decoder_one_shot_complete_packet() {
    let mut dec = Sample::decoder();
    dec.feed(&encoded(1, 2, 3));

    let first = dec.next().expect("expected Ready").expect("Ok");
    assert_eq!(first, Sample { a: 1, b: 2, c: 3 });

    assert!(dec.next().is_none(), "empty buffer must yield None");
    assert!(dec.buffered().is_empty());
}

#[test]
/// Three back-to-back packets fed as a single blob must emerge one at a
/// time via successive `next()` calls, in order.
fn decoder_multiple_packets_one_feed() {
    let mut blob = Vec::new();
    blob.extend(encoded(1, 2, 3));
    blob.extend(encoded(4, 5, 6));
    blob.extend(encoded(7, 8, 9));

    let mut dec = Sample::decoder();
    dec.feed(&blob);

    let got: Vec<Sample> = core::iter::from_fn(|| dec.next())
        .map(|r| r.expect("Ok"))
        .collect();

    assert_eq!(got.len(), 3);
    assert_eq!(got[0], Sample { a: 1, b: 2, c: 3 });
    assert_eq!(got[1], Sample { a: 4, b: 5, c: 6 });
    assert_eq!(got[2], Sample { a: 7, b: 8, c: 9 });
    assert!(dec.buffered().is_empty());
}

#[test]
/// Feed one byte at a time. Every `next()` should return `None` until the
/// full 9-byte packet has arrived, then `Ready`. Repeat across a stream
/// of 5 packets.
fn decoder_byte_by_byte_preserves_progress() {
    let packets = [
        Sample {
            a: 10,
            b: 20,
            c: 30,
        },
        Sample {
            a: 40,
            b: 50,
            c: 60,
        },
        Sample {
            a: 70,
            b: 80,
            c: 90,
        },
        Sample {
            a: 100,
            b: 110,
            c: 120,
        },
        Sample {
            a: 130,
            b: 140,
            c: 150,
        },
    ];
    let mut blob = Vec::new();
    for p in &packets {
        blob.extend(p.encode_frame());
    }

    let mut dec = Sample::decoder();
    let mut got = Vec::new();
    for &byte in &blob {
        dec.feed(&[byte]);
        for r in dec.by_ref() {
            got.push(r.expect("Ok"));
        }
    }

    assert_eq!(got.len(), packets.len());
    for (i, p) in packets.iter().enumerate() {
        assert_eq!(&got[i], p);
    }
    assert!(dec.buffered().is_empty());
}

#[test]
/// Feeding a truncated first packet and nothing else must yield `None`
/// from `next()` (NeedMore) without losing any buffered bytes. Completing
/// the packet later must produce `Ready` with the correct value.
fn decoder_truncated_then_resumed() {
    let full = encoded(42, 43, 44);
    let (head, tail) = full.split_at(5);

    let mut dec = Sample::decoder();
    dec.feed(head);
    assert!(dec.next().is_none(), "truncated feed must yield NeedMore");
    assert_eq!(dec.buffered().len(), 5, "no bytes lost on NeedMore");

    dec.feed(tail);
    let got = dec.next().expect("expected Ready").expect("Ok");
    assert_eq!(
        got,
        Sample {
            a: 42,
            b: 43,
            c: 44
        }
    );
    assert!(dec.buffered().is_empty());
}

#[test]
/// Feed a multi-packet stream in arbitrary odd chunks and assert no
/// packet is lost, no duplicate emission, and no buffer residue.
fn decoder_irregular_chunks() {
    let packets: Vec<Sample> = (0u8..7)
        .map(|i| Sample {
            a: i,
            b: i.wrapping_add(1),
            c: i.wrapping_add(2),
        })
        .collect();
    let mut blob = Vec::new();
    for p in &packets {
        blob.extend(p.encode_frame());
    }

    let chunk_sizes = [1usize, 4, 2, 3, 9, 5, 7, 11, 2, 8, 1, 6, 3];

    let mut dec = Sample::decoder();
    let mut got = Vec::new();
    let mut cursor = 0usize;
    let mut chunk_i = 0usize;
    while cursor < blob.len() {
        let want = chunk_sizes[chunk_i % chunk_sizes.len()];
        let end = (cursor + want).min(blob.len());
        dec.feed(&blob[cursor..end]);
        cursor = end;
        chunk_i += 1;
        for r in dec.by_ref() {
            got.push(r.expect("Ok"));
        }
    }

    assert_eq!(got.len(), packets.len());
    assert_eq!(got, packets);
    assert!(dec.buffered().is_empty());
}
