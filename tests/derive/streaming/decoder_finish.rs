//! [`tinyklv::Decoder::finish`]: what ending a stream returns, by what the decoder still holds
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use tinyklv::dec::binary as decb;
use tinyklv::decoder::DecodeIterError;
use tinyklv::enc::binary as encb;
use tinyklv::prelude::*;

/// A reading behind the sentinel `SENSOR`, both of its fields required
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"SENSOR",
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct Sensor {
    #[klv(key = 0x01, dec = decb::u8, enc = *encb::u8)]
    seq: u8,
    #[klv(key = 0x02, dec = decb::be_u16, enc = *encb::be_u16)]
    temp: u16,
}

/// [`Sensor`] implementation
impl Sensor {
    /// The reading that [`Self::SEQ`] and [`Self::TEMP`] encode
    const READING: Self = Self {
        seq: 42,
        temp: 2350,
    };

    /// A packet's sentinel and its length, that of [`Self::SEQ`] and [`Self::TEMP`]
    #[rustfmt::skip]
    const HEADER: [u8; 7] = [
        // sentinel
        b'S', b'E', b'N', b'S', b'O', b'R',
        // packet length
        0x07,
    ];

    /// The `seq` item
    #[rustfmt::skip]
    const SEQ: [u8; 3] = [
        // seq = 42
        0x01, 0x01, 0x2A,
    ];

    /// The `temp` item
    #[rustfmt::skip]
    const TEMP: [u8; 4] = [
        // temp = 2350
        0x02, 0x02, 0x09, 0x2E,
    ];
}

#[test]
/// A stream that `next` decoded whole ends in `Ok(None)`
fn finish_after_the_last_packet_is_none() {
    let mut dec = Sensor::decoder();
    dec.feed(&Sensor::HEADER);
    dec.feed(&Sensor::SEQ);
    dec.feed(&Sensor::TEMP);
    assert_eq!(dec.next(), Some(Sensor::READING));
    assert_eq!(dec.next(), None);
    assert_eq!(dec.finish().expect("the stream ends cleanly"), None);
}

#[test]
/// A resumed partial that holds every required field, with no byte left, ends in its value
fn finish_completes_a_resumed_partial() {
    let mut dec = Sensor::decoder();
    dec.feed(&Sensor::SEQ);
    dec.feed(&Sensor::TEMP);
    let resumed = dec.next_resume();
    assert!(
        matches!(resumed, Err(DecodeIterError::NeedMore)),
        "{resumed:?}"
    );
    let reading = dec.finish().expect("the partial finalizes");
    assert_eq!(reading, Some(Sensor::READING));
}

#[test]
/// A packet cut short leaves its bytes in the decoder, which `finish` reports as truncated
fn finish_with_bytes_left_is_truncated() {
    let mut dec = Sensor::decoder();
    dec.feed(&Sensor::HEADER);
    dec.feed(&Sensor::SEQ);
    assert_eq!(dec.next(), None);
    let result = dec.finish();
    assert_eq!(
        result.as_ref().err().and_then(DecodeIterError::kind),
        Some(ErrorKind::Truncated),
        "{result:?}"
    );
    let Err(DecodeIterError::Malformed(error)) = result else {
        panic!("expected Malformed, got {result:?}");
    };
    assert_eq!(
        error.to_string(),
        "invalid end of stream (bytes are left in the decoder)"
    );
}

#[test]
/// A resumed partial missing a required field ends in an error naming that field
fn finish_on_a_partial_missing_a_field_is_missing_required() {
    let mut dec = Sensor::decoder();
    dec.feed(&Sensor::SEQ);
    let resumed = dec.next_resume();
    assert!(
        matches!(resumed, Err(DecodeIterError::NeedMore)),
        "{resumed:?}"
    );
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
        "invalid `Sensor::temp` value (required: missing from the packet, or its decoder \
         rejected it, which `strict` reports; to allow this, make the field an `Option` or give \
         it a `default`)",
    );
}
