//! End-to-end streaming tests for [`tinyklv::Decoder`]
//!
//! Feeds a byte stream through `Decoder<T>::feed` + `next()` in several
//! granularities (one packet at a time, one byte at a time, random chunks)
//! and asserts every packet emerges with no lost progress

// --------------------------------------------------
// local
// --------------------------------------------------
use tinyklv::dec::binary as decb;
use tinyklv::enc::binary as encb;
use tinyklv::prelude::*;

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

/// Build a sentinel-framed `Sample` byte vector from raw field values
///
/// Constructs a `Sample` and calls `encode_frame`, producing the canonical
/// `sentinel + body_len + body` wire format used by all streaming tests
fn encoded(a: u8, b: u8, c: u8) -> Vec<u8> {
    // framed form: sentinel + len + body
    Sample { a, b, c }.encode_frame()
}

#[test]
/// One complete packet fed in one shot must yield exactly one `Ready`
/// from `next()`, then `None` (buffer empty)
fn decoder_one_shot_complete_packet() {
    let mut dec = Sample::decoder();
    dec.feed(&encoded(1, 2, 3));

    let first = dec.next().expect("expected Ready");
    assert_eq!(first, Sample { a: 1, b: 2, c: 3 });

    assert!(dec.next().is_none(), "empty buffer must yield None");
    assert!(dec.buffered().is_empty());
}

#[test]
/// Three back-to-back packets fed as a single blob must emerge one at a
/// time via successive `next()` calls, in order
fn decoder_multiple_packets_one_feed() {
    let mut blob = Vec::new();
    blob.extend(encoded(1, 2, 3));
    blob.extend(encoded(4, 5, 6));
    blob.extend(encoded(7, 8, 9));

    let mut dec = Sample::decoder();
    dec.feed(&blob);

    let got: Vec<Sample> = dec.iter().collect();

    assert_eq!(got.len(), 3);
    assert_eq!(got[0], Sample { a: 1, b: 2, c: 3 });
    assert_eq!(got[1], Sample { a: 4, b: 5, c: 6 });
    assert_eq!(got[2], Sample { a: 7, b: 8, c: 9 });
    assert!(dec.buffered().is_empty());
}

#[test]
/// Feed one byte at a time. Every `next()` should return `None` until the
/// full 9-byte packet has arrived, then `Ready`. Repeat across a stream
/// of 5 packets
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

    blob.iter().for_each(|&byte| {
        dec.feed(&[byte]);
        dec.iter().for_each(|r| got.push(r));
    });

    assert_eq!(got.len(), packets.len());
    for (i, p) in packets.iter().enumerate() {
        assert_eq!(&got[i], p);
    }
    assert!(dec.buffered().is_empty());
}

#[test]
/// Feeding a truncated first packet and nothing else must yield `None`
/// from `next()` (`NeedMore`) without losing any buffered bytes. Completing
/// the packet later must produce `Ready` with the correct value
fn decoder_truncated_then_resumed() {
    let full = encoded(42, 43, 44);
    let (head, tail) = full.split_at(5);

    let mut dec = Sample::decoder();
    dec.feed(head);
    assert!(dec.next().is_none(), "truncated feed must yield NeedMore");
    assert_eq!(dec.buffered().len(), 5, "no bytes lost on NeedMore");

    dec.feed(tail);
    let got = dec.next().expect("complete feed yields Some");
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
/// packet is lost, no duplicate emission, and no buffer residue
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
        for r in dec.iter() {
            got.push(r);
        }
    }

    assert_eq!(got.len(), packets.len());
    assert_eq!(got, packets);
    assert!(dec.buffered().is_empty());
}

#[test]
/// Junk bytes (zeros, 0xFF, arbitrary patterns) between sentinel-framed
/// packets are skipped by the sentinel seeker. All packets decode in order
fn decoder_skips_junk_between_packets() {
    let mut blob = Vec::new();
    // junk before first sentinel
    blob.extend_from_slice(&[0x00, 0x00, 0xFF, 0xDE, 0xAD]);
    blob.extend(encoded(1, 2, 3));
    // junk between first and second
    blob.extend_from_slice(&[0xFF, 0x00, 0xBE, 0xEF, 0x00, 0x00]);
    blob.extend(encoded(4, 5, 6));
    // junk between second and third
    blob.extend_from_slice(&[0x00, 0xCA, 0xFE]);
    blob.extend(encoded(7, 8, 9));

    let mut dec = Sample::decoder();
    dec.feed(&blob);

    let got: Vec<Sample> = dec.iter().collect();

    assert_eq!(got.len(), 3);
    assert_eq!(got[0], Sample { a: 1, b: 2, c: 3 });
    assert_eq!(got[1], Sample { a: 4, b: 5, c: 6 });
    assert_eq!(got[2], Sample { a: 7, b: 8, c: 9 });
}

#[test]
/// Byte-at-a-time feeding through junk-interspersed packets. The sentinel
/// seeker finds each packet despite single-byte feeds through noise
fn decoder_skips_junk_byte_by_byte() {
    let mut blob = Vec::new();
    // junk before first sentinel
    blob.extend_from_slice(&[0x00, 0x00, 0xFF, 0xDE, 0xAD]);
    blob.extend(encoded(1, 2, 3));
    // junk between first and second
    blob.extend_from_slice(&[0xFF, 0x00, 0xBE, 0xEF, 0x00, 0x00]);
    blob.extend(encoded(4, 5, 6));
    // junk between second and third
    blob.extend_from_slice(&[0x00, 0xCA, 0xFE]);
    blob.extend(encoded(7, 8, 9));

    let mut dec = Sample::decoder();
    let mut got = Vec::new();
    for &byte in &blob {
        dec.feed(&[byte]);
        for r in dec.iter() {
            got.push(r);
        }
    }

    assert_eq!(got.len(), 3);
    assert_eq!(got[0], Sample { a: 1, b: 2, c: 3 });
    assert_eq!(got[1], Sample { a: 4, b: 5, c: 6 });
    assert_eq!(got[2], Sample { a: 7, b: 8, c: 9 });
}

#[test]
/// A malformed packet (missing required fields) followed by a good one
/// The decoder surfaces the error and recovers to decode the next packet
fn decoder_malformed_then_good_recovers() {
    let mut blob = Vec::new();
    // malformed: sentinel + length + only field a (b and c are absent)
    blob.extend_from_slice(b"SMPL");
    blob.push(3); // body length = 3 bytes
    blob.extend_from_slice(&[0x01, 0x01, 0xAA]); // only field a = 0xAA
                                                 // good packet following the malformed one
    blob.extend(encoded(10, 20, 30));

    let mut dec = Sample::decoder();
    dec.feed(&blob);

    let first = dec.next();
    assert!(
        first.is_none(),
        "malformed packet must halt decoding and yield None"
    );

    let second = dec.next().expect("expected Some for good packet");
    assert_eq!(
        second,
        Sample {
            a: 10,
            b: 20,
            c: 30
        }
    );
}

#[test]
/// `finish()` on a fresh decoder with no bytes and no partial yields an
/// error naming the first missing required field
fn decoder_finish_on_empty() {
    let dec = Sample::decoder();
    let result = dec.finish();
    assert!(result.is_err());
}

#[test]
/// Feed 3 packets in a single blob, then drain via `for pkt in &mut dec`
/// (the `IntoIterator` impl). All 3 packets must emerge in order
fn decoder_into_iter_pattern() {
    let mut blob = Vec::new();
    blob.extend(encoded(5, 6, 7));
    blob.extend(encoded(8, 9, 10));
    blob.extend(encoded(11, 12, 13));

    let mut dec = Sample::decoder();
    dec.feed(&blob);

    let mut got = Vec::new();
    for pkt in &mut dec {
        got.push(pkt);
    }

    assert_eq!(got.len(), 3);
    assert_eq!(got[0], Sample { a: 5, b: 6, c: 7 });
    assert_eq!(got[1], Sample { a: 8, b: 9, c: 10 });
    assert_eq!(
        got[2],
        Sample {
            a: 11,
            b: 12,
            c: 13
        }
    );
    assert!(dec.buffered().is_empty());
}
