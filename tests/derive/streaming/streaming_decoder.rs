//! End-to-end streaming tests for [`tinyklv::Decoder`]
//!
//! Feeds a byte stream through `Decoder<T>::feed` + `next()` in several
//! granularities (one packet at a time, one byte at a time, random chunks)
//! and asserts every packet emerges with no lost progress. Includes regression
//! coverage for the malformed-at-tail panic (out-of-bounds error slice on the
//! final packet in the buffer) and correctness checks for the `finish` path
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use tinyklv::ErrorKind;
use tinyklv::dec::ber as decber;
use tinyklv::dec::binary as decb;
use tinyklv::dec::string as decs;
use tinyklv::decoder::DecodeIterError;
use tinyklv::enc::ber as encber;
use tinyklv::enc::binary as encb;
use tinyklv::prelude::*;
use tinyklv::traits::PartialIterator;

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
    let mut out = Vec::new();
    Sample { a, b, c }.encode_frame(&mut out);
    out
}

#[test]
/// One complete packet fed in one shot must yield exactly one `Ready`
/// from `next()`, then `None` (buffer empty)
fn decoder_one_shot_complete_packet() {
    // --------------------------------------------------
    // feed one complete packet
    // --------------------------------------------------
    let mut dec = Sample::decoder();
    dec.feed(&encoded(1, 2, 3));
    // --------------------------------------------------
    // first next must yield Ready; second must yield None
    // --------------------------------------------------
    let first = dec.next().expect("expected Ready");
    assert_eq!(first, Sample { a: 1, b: 2, c: 3 });
    assert!(dec.next().is_none(), "empty buffer must yield None");
    assert!(dec.buffered().is_empty());
}

#[test]
/// Three back-to-back packets fed as a single blob must emerge one at a
/// time via successive `next()` calls, in order
fn decoder_multiple_packets_one_feed() {
    // --------------------------------------------------
    // build a single blob of three back-to-back packets
    // --------------------------------------------------
    let mut blob = Vec::new();
    blob.extend(encoded(1, 2, 3));
    blob.extend(encoded(4, 5, 6));
    blob.extend(encoded(7, 8, 9));
    // --------------------------------------------------
    // feed and drain via iter
    // --------------------------------------------------
    let mut dec = Sample::decoder();
    dec.feed(&blob);
    let got: Vec<Sample> = dec.iter().collect();
    // --------------------------------------------------
    // assert all three packets decoded in order
    // --------------------------------------------------
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
    // --------------------------------------------------
    // build five packets and concatenate into one blob
    // --------------------------------------------------
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
        p.encode_frame(&mut blob);
    }
    // --------------------------------------------------
    // feed one byte at a time; drain after each feed
    // --------------------------------------------------
    let mut dec = Sample::decoder();
    let mut got = Vec::new();
    blob.iter().for_each(|&byte| {
        dec.feed(&[byte]);
        dec.iter().for_each(|r| got.push(r));
    });
    // --------------------------------------------------
    // assert all packets decoded in order with no residue
    // --------------------------------------------------
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
    // --------------------------------------------------
    // split the packet into two halves
    // --------------------------------------------------
    let full = encoded(42, 43, 44);
    let (head, tail) = full.split_at(5);
    // --------------------------------------------------
    // feed truncated head; must yield None, no bytes lost
    // --------------------------------------------------
    let mut dec = Sample::decoder();
    dec.feed(head);
    assert!(dec.next().is_none(), "truncated feed must yield NeedMore");
    assert_eq!(dec.buffered().len(), 5, "no bytes lost on NeedMore");
    // --------------------------------------------------
    // feed remaining tail; must yield the complete packet
    // --------------------------------------------------
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
    // --------------------------------------------------
    // build seven packets concatenated into one blob
    // --------------------------------------------------
    let packets: Vec<Sample> = (0u8..7)
        .map(|i| Sample {
            a: i,
            b: i.wrapping_add(1),
            c: i.wrapping_add(2),
        })
        .collect();
    let mut blob = Vec::new();
    for p in &packets {
        p.encode_frame(&mut blob);
    }
    // --------------------------------------------------
    // feed in irregular chunk sizes; drain after each feed
    // --------------------------------------------------
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
    // --------------------------------------------------
    // assert all packets decoded in order with no residue
    // --------------------------------------------------
    assert_eq!(got.len(), packets.len());
    assert_eq!(got, packets);
    assert!(dec.buffered().is_empty());
}

#[test]
/// Junk bytes (zeros, 0xFF, arbitrary patterns) between sentinel-framed
/// packets are skipped by the sentinel seeker. All packets decode in order
fn decoder_skips_junk_between_packets() {
    // --------------------------------------------------
    // build a blob with junk between each packet
    // --------------------------------------------------
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
    // --------------------------------------------------
    // feed all at once and drain via iter
    // --------------------------------------------------
    let mut dec = Sample::decoder();
    dec.feed(&blob);
    let got: Vec<Sample> = dec.iter().collect();
    // --------------------------------------------------
    // assert all three packets decoded in order
    // --------------------------------------------------
    assert_eq!(got.len(), 3);
    assert_eq!(got[0], Sample { a: 1, b: 2, c: 3 });
    assert_eq!(got[1], Sample { a: 4, b: 5, c: 6 });
    assert_eq!(got[2], Sample { a: 7, b: 8, c: 9 });
}

#[test]
/// Byte-at-a-time feeding through junk-interspersed packets. The sentinel
/// seeker finds each packet despite single-byte feeds through noise
fn decoder_skips_junk_byte_by_byte() {
    // --------------------------------------------------
    // build a blob with junk between each packet
    // --------------------------------------------------
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
    // --------------------------------------------------
    // feed one byte at a time; drain after each feed
    // --------------------------------------------------
    let mut dec = Sample::decoder();
    let mut got = Vec::new();
    for &byte in &blob {
        dec.feed(&[byte]);
        for r in dec.iter() {
            got.push(r);
        }
    }
    // --------------------------------------------------
    // assert all three packets decoded in order
    // --------------------------------------------------
    assert_eq!(got.len(), 3);
    assert_eq!(got[0], Sample { a: 1, b: 2, c: 3 });
    assert_eq!(got[1], Sample { a: 4, b: 5, c: 6 });
    assert_eq!(got[2], Sample { a: 7, b: 8, c: 9 });
}

#[test]
/// A malformed packet (missing required fields) followed by a good one
/// The decoder surfaces the error and recovers to decode the next packet
fn decoder_malformed_then_good_recovers() {
    // --------------------------------------------------
    // build a malformed packet followed by a good one
    // --------------------------------------------------
    let mut blob = Vec::new();
    // malformed: sentinel + length + only field a (b and c are absent)
    blob.extend_from_slice(b"SMPL");
    blob.push(3); // body length = 3 bytes
    blob.extend_from_slice(&[0x01, 0x01, 0xAA]); // only field a = 0xAA
    // good packet following the malformed one
    blob.extend(encoded(10, 20, 30));
    // --------------------------------------------------
    // feed both; first next must yield None (malformed)
    // --------------------------------------------------
    let mut dec = Sample::decoder();
    dec.feed(&blob);
    let first = dec.next();
    assert!(
        first.is_none(),
        "malformed packet must halt decoding and yield None"
    );
    // --------------------------------------------------
    // second next must recover and yield the good packet
    // --------------------------------------------------
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
/// A malformed packet (missing required fields) sitting at the very end of the buffer, with
/// no trailing bytes, must surface as `None` without panicking
///
/// Regression: in fresh mode the `NeedMore` -> `finalize` error branch built its error-context
/// slice from the post-advance cursor (`buf[head..head + consumed]`), which ran past the buffer
/// end and panicked out-of-bounds whenever the malformed packet was the buffer tail (fewer than
/// `consumed` bytes remaining). `decoder_malformed_then_good_recovers` masks this because the
/// trailing good packet leaves enough bytes to slice; this case removes them
fn decoder_malformed_at_tail_does_not_panic() {
    // --------------------------------------------------
    // sentinel + len + only field a; b and c absent; nothing after
    // --------------------------------------------------
    let mut blob = Vec::new();
    blob.extend_from_slice(b"SMPL");
    blob.push(3);
    blob.extend_from_slice(&[0x01, 0x01, 0xAA]);
    // --------------------------------------------------
    // feed; must not panic: missing b/c -> finalize fails -> None
    // --------------------------------------------------
    let mut dec = Sample::decoder();
    dec.feed(&blob);
    assert!(
        dec.next().is_none(),
        "malformed tail packet must yield None, not panic"
    );
}

#[test]
/// A fresh-mode seek that cannot find the sentinel yet must report `NeedMore`
/// (not `Malformed`) and keep only the last `sentinel_len - 1` bytes, the ones
/// that may still begin a sentinel, so stray bytes never pile up
///
/// Regression: `next_fresh` previously returned `Malformed` on any `seek_sentinel`
/// failure, conflating "the sentinel has not arrived in the buffer yet" with genuine
/// corruption and violating `Malformed`'s documented contract (which promises the
/// offending bytes were already advanced past). A sentinel-framed stream has no
/// corrupt-sentinel case: absent framing simply means more bytes are needed
fn decoder_fresh_seek_miss_is_needmore() {
    use tinyklv::decoder::DecodeIterError;
    use tinyklv::traits::PartialIterator;
    // --------------------------------------------------
    // junk with no "SMPL" sentinel anywhere
    // --------------------------------------------------
    let junk: &[u8] = &[0x00, 0xFF, 0x42, 0x13, 0x37];
    let mut dec = Sample::decoder();
    // --------------------------------------------------
    // each seek misses: must be NeedMore, and the buffer stays at the last 3 bytes
    // --------------------------------------------------
    for _ in 0..3 {
        dec.feed(junk);
        let result = dec.next_fresh();
        assert!(
            matches!(result, Err(DecodeIterError::NeedMore)),
            "missing sentinel must report NeedMore, got {result:?}"
        );
        assert_eq!(
            dec.buffered(),
            &junk[2..],
            "seek miss must keep only the bytes that may begin a sentinel"
        );
    }
    // --------------------------------------------------
    // once a real packet arrives the junk prefix is skipped and it decodes
    // --------------------------------------------------
    dec.feed(&encoded(10, 20, 30));
    let pkt = dec
        .next()
        .expect("packet must decode after the sentinel arrives");
    assert_eq!(
        pkt,
        Sample {
            a: 10,
            b: 20,
            c: 30
        }
    );
}

/// A packet behind the 4-byte sentinel `06 0E 2B 34` with a BER packet length
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"\x06\x0E\x2B\x34",
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decber::ber_length, enc = encber::ber_length),
)]
struct Heading {
    #[klv(key = 0x01, dec = decb::be_u16, enc = *encb::be_u16)]
    degrees: u16,
}

/// [`Heading`] implementation
impl Heading {
    /// A packet whose BER length overflows `usize`
    #[rustfmt::skip]
    const OVERFLOWING: [u8; 14] = [
        // sentinel
        0x06, 0x0E, 0x2B, 0x34,
        // long-form length of 9 bytes, 2^64
        0x89, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];
}

#[test]
/// One-shot, a malformed packet length is returned with the cursor back at its sentinel, past the
/// junk before it
fn decode_frame_leaves_a_malformed_length_at_its_sentinel() {
    let wire = [&[0x00, 0xFF][..], &Heading::OVERFLOWING].concat();
    let mut cursor = &wire[..];
    let result = Heading::decode_frame(&mut cursor);
    assert_eq!(
        result.as_ref().err().and_then(ErrorKind::of),
        Some(ErrorKind::MalformedValue),
        "{result:?}"
    );
    assert_eq!(cursor, Heading::OVERFLOWING);
}

#[test]
/// One-shot, a packet length the input cuts or its decoder rejects is named as the struct's
/// packet length first, then as the BER length decoder names it
fn decode_frame_names_a_failed_length_as_its_packets() {
    // the sentinel, then a long-form length of 2 bytes cut after 1
    let cut = [&Heading::OVERFLOWING[..4], &[0x82, 0x01]].concat();
    for wire in [&cut[..], &Heading::OVERFLOWING] {
        let display = Heading::decode_frame(&mut &wire[..])
            .expect_err("a failed length")
            .to_string();
        let mut lines = display.lines();
        assert_eq!(
            lines.next(),
            Some("invalid `Heading` packet length"),
            "{display}"
        );
        assert_eq!(lines.next(), Some("invalid BER length value"), "{display}");
    }
}

#[test]
/// A sentinel whose BER length overflows `usize` is a malformed packet: the decoder returns its
/// error once, then searches on from one byte into its sentinel, so every good packet fed after
/// it decodes
fn decoder_reports_a_malformed_length_once() {
    let bad = Heading::OVERFLOWING;
    #[rustfmt::skip]
    let good = [
        // sentinel, length 4, degrees = 7
        0x06, 0x0E, 0x2B, 0x34, 0x04, 0x01, 0x02, 0x00, 0x07,
        // sentinel, length 4, degrees = 8
        0x06, 0x0E, 0x2B, 0x34, 0x04, 0x01, 0x02, 0x00, 0x08,
    ];
    let mut dec = Heading::decoder();
    dec.feed(&bad);
    let result = dec.next_fresh();
    assert_eq!(
        result.as_ref().err().and_then(DecodeIterError::kind),
        Some(ErrorKind::MalformedValue),
        "{result:?}"
    );
    dec.feed(&good);
    let got: Vec<Heading> = dec.iter().collect();
    assert_eq!(got, [Heading { degrees: 7 }, Heading { degrees: 8 }]);
    assert!(dec.buffered().is_empty());
}

/// A packet behind the sentinel `AA` whose packet and item lengths are winnow's own `be_u16`,
/// which leaves the cursor in place when it runs out of bytes
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"\xAA",
    key(dec = decb::u8),
    len(dec = Ping::be_u16_len),
    allow_unimplemented_encode,
)]
struct Ping {
    #[klv(key = 0x01, dec = decb::u8)]
    id: u8,
    #[klv(key = 0x02, dec = decs::to_string_utf8, size(lengthed))]
    note: Option<String>,
}

/// [`Ping`] implementation
impl Ping {
    /// Decodes a big-endian `u16` length
    ///
    /// # Arguments
    ///
    /// * `input` - the byte stream positioned at the length; left in place when it holds fewer
    ///   than two bytes
    ///
    /// # Returns
    ///
    /// The length, or winnow's error on a short read
    fn be_u16_len(input: &mut &[u8]) -> tinyklv::Result<usize> {
        winnow::binary::be_u16.map(usize::from).parse_next(input)
    }

    /// Feeds `first` then `rest` to a fresh decoder, collecting the packets after each feed
    ///
    /// # Arguments
    ///
    /// * `first` - the first chunk
    /// * `rest` - the second chunk
    ///
    /// # Returns
    ///
    /// Every packet decoded, and the bytes left buffered
    fn fed_in_two(first: &[u8], rest: &[u8]) -> (Vec<Self>, Vec<u8>) {
        let mut dec = Self::decoder();
        dec.feed(first);
        let mut got: Vec<Self> = dec.iter().collect();
        dec.feed(rest);
        got.extend(dec.iter());
        (got, dec.buffered().to_vec())
    }
}

#[test]
/// A packet length cut by a chunk boundary waits for the rest, even when its decoder stops short
/// of the end of the input: fed `AA 00` then the rest, the packet decodes
fn decoder_waits_for_a_length_its_decoder_leaves_unread() {
    #[rustfmt::skip]
    let wire = [
        // sentinel, packet length 4
        0xAA, 0x00, 0x04,
        // id = 7: key, item length 1, value
        0x01, 0x00, 0x01, 0x07,
    ];
    let (got, left) = Ping::fed_in_two(&wire[..2], &wire[2..]);
    assert_eq!(got, [Ping { id: 7, note: None }]);
    assert!(left.is_empty());
}

#[test]
/// A packet whose note carries the bytes of another packet, cut inside its packet length, decodes
/// as the outer packet: the sentinel inside the note is never searched for
fn decoder_never_yields_a_packet_forged_in_a_body() {
    #[rustfmt::skip]
    let forged = [
        // sentinel, packet length 4, id = 9
        0xAA, 0x00, 0x04, 0x01, 0x00, 0x01, 0x09,
    ];
    #[rustfmt::skip]
    let wire = [
        // sentinel, packet length 14
        &[0xAA, 0x00, 0x0E][..],
        // note: key, item length 7, the forged packet
        &[0x02, 0x00, 0x07], &forged,
        // id = 7: key, item length 1, value
        &[0x01, 0x00, 0x01, 0x07],
    ]
    .concat();
    let (got, left) = Ping::fed_in_two(&wire[..2], &wire[2..]);
    let outer = Ping {
        id: 7,
        note: Some(String::from_utf8_lossy(&forged).into_owned()),
    };
    assert_eq!(got, [outer]);
    assert!(left.is_empty());
}

/// A packet behind the 4-byte sentinel `06 0E 2B 34` with a two-byte big-endian packet length
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"\x06\x0E\x2B\x34",
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::be_u16_as_usize, enc = encb::be_u16_from_usize),
)]
struct Bearing {
    #[klv(key = 0x01, dec = decb::be_u16, enc = *encb::be_u16)]
    degrees: u16,
}

#[test]
/// A packet whose fixed-width length is cut between its two bytes waits for the rest: fed one
/// byte at a time, the packet decodes once, and nothing is left buffered
fn decoder_waits_for_a_cut_fixed_width_length() {
    #[rustfmt::skip]
    let wire = [
        // sentinel
        0x06, 0x0E, 0x2B, 0x34,
        // packet length 5, as two bytes
        0x00, 0x05,
        // degrees = 7: key, two-byte length 2, value
        0x01, 0x00, 0x02, 0x00, 0x07,
    ];
    let mut dec = Bearing::decoder();
    let mut got = Vec::new();
    for byte in wire {
        dec.feed(&[byte]);
        got.extend(dec.iter());
    }
    assert_eq!(got, [Bearing { degrees: 7 }]);
    assert!(dec.buffered().is_empty());
}

#[test]
/// `finish()` on a fresh decoder with no bytes and no partial yields an
/// error of kind `MissingRequired`, naming the first missing required field
fn decoder_finish_on_empty() {
    let dec = Sample::decoder();
    let result = dec.finish();
    assert_eq!(
        result.as_ref().err().and_then(DecodeIterError::kind),
        Some(ErrorKind::MissingRequired),
        "{result:?}"
    );
    let Err(DecodeIterError::Malformed(error)) = result else {
        panic!("expected Malformed, got {result:?}");
    };
    assert_eq!(
        error.to_string(),
        "invalid `Sample::a` value (required, and missing from the packet; to allow this, make \
         the field an `Option` or give it a `default`)",
    );
}

/// A sentinel packet of one field that rejects a key no field declares
#[derive(Klv, Debug, PartialEq)]
#[klv(
    sentinel = b"STRC",
    key(dec = decb::u8),
    len(dec = decb::u8_as_usize),
    deny_unknown_keys,
    allow_unimplemented_encode,
)]
struct StrictSample {
    #[klv(key = 0x01, dec = decb::u8)]
    a: u8,
}

#[test]
/// A body `next_resume` rejects is `Malformed`, with the kind and the label of the derive's
/// streaming error, which its display prints once after `invalid `
fn decoder_next_resume_error_carries_its_kind() {
    let mut dec = StrictSample::decoder();
    #[rustfmt::skip]
    dec.feed(&[
        // a = 1
        0x01, 0x01, 0x01,
        // key 0x09, which no field declares
        0x09, 0x01, 0x00,
    ]);
    let error = dec.next_resume().expect_err("an unknown key is rejected");
    assert_eq!(error.kind(), Some(ErrorKind::UnknownKey), "{error:?}");
    let DecodeIterError::Malformed(error) = error else {
        panic!("expected Malformed, got {error:?}");
    };
    assert_eq!(
        error.to_string(),
        "invalid key (expected one of the keys defined on `StrictSample`; to turn this off, \
         remove `deny_unknown_keys`)",
    );
}

#[test]
/// Feed 3 packets in a single blob, then drain via `for pkt in &mut dec`
/// (the `IntoIterator` impl). All 3 packets must emerge in order
fn decoder_into_iter_pattern() {
    // --------------------------------------------------
    // build a single blob of three packets
    // --------------------------------------------------
    let mut blob = Vec::new();
    blob.extend(encoded(5, 6, 7));
    blob.extend(encoded(8, 9, 10));
    blob.extend(encoded(11, 12, 13));
    // --------------------------------------------------
    // feed and drain via IntoIterator on &mut dec
    // --------------------------------------------------
    let mut dec = Sample::decoder();
    dec.feed(&blob);
    let mut got = Vec::new();
    for pkt in &mut dec {
        got.push(pkt);
    }
    // --------------------------------------------------
    // assert all three packets decoded in order
    // --------------------------------------------------
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
