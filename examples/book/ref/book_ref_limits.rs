// ANCHOR: all
//! book reference - limits: what tinyklv decodes as the bytes say,
//! and does not repair
use tinyklv::dec::ber as decber;
use tinyklv::dec::binary as decb;
use tinyklv::dec::string as decs;
use tinyklv::enc::ber as encber;
use tinyklv::enc::binary as encb;
use tinyklv::enc::string as encs;
use tinyklv::prelude::*;
use tinyklv::winnow::error::{ContextError, StrContext};

#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(
        dec = decb::u8,
        enc = encb::u8,
    ),
    len(
        dec = decber::ber_length,
        enc = encber::ber_length,
    ),
)]
/// Three optional fields, BER lengths
struct Three {
    #[klv(
        key = 0x01,
        dec = decb::u8,
        enc = *encb::u8,
    )]
    a: Option<u8>,

    #[klv(
        key = 0x02,
        dec = decb::u8,
        enc = *encb::u8,
    )]
    b: Option<u8>,

    #[klv(
        key = 0x03,
        dec = decb::u8,
        enc = *encb::u8,
    )]
    c: Option<u8>,
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    sentinel = b"\xAA",
    key(
        dec = decb::u8,
        enc = encb::u8,
    ),
    len(
        dec = packet_len,
        enc = encb::u8_from_usize,
    ),
    strict,
)]
/// One required field, one optional, in a packet of at most
/// 64 bytes
struct Frame {
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
    b: Option<u8>,
}

// ANCHOR: packet_len
/// The longest packet this protocol sends
const MAX_PACKET: usize = 64;

/// Reads a length, which past `MAX_PACKET` is never valid
fn packet_len(input: &mut &[u8]) -> tinyklv::Result<usize> {
    let len = decb::u8_as_usize(input)?;
    if len > MAX_PACKET {
        let mut error = ContextError::from_input(input);
        error.push(StrContext::Label("packet length past MAX_PACKET"));
        return Err(ErrorKind::MalformedValue.tag(error));
    }
    Ok(len)
}
// ANCHOR_END: packet_len

#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(
        dec = decb::u8,
        enc = encb::u8,
    ),
    len(
        dec = decb::u8_as_usize,
        enc = encb::u8_from_usize,
    ),
    break_on = 0xFF,
)]
/// Ends at key FF
struct Reading {
    #[klv(
        key = 0x01,
        dec = decb::u8,
        enc = *encb::u8,
    )]
    temp: Option<u8>,
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    keylen(
        dec = dec_header,
        enc = enc_header,
    ),
    break_on = stop_at_dead,
)]
/// Key 10 has no length; every other key has one
struct Peeked {
    #[klv(
        key = 0x01,
        dec = decb::u8,
        enc = *encb::u8,
    )]
    a: Option<u8>,
}

/// Key 10 has no length; every other key has a 1-byte one
fn dec_header(input: &mut &[u8]) -> tinyklv::Result<(u8, Framing)> {
    let key = decb::u8(input)?;
    if key == 0x10 {
        return Ok((key, Framing::Inline));
    }
    Ok((key, Framing::Sliced(decb::u8_as_usize(input)?)))
}

/// Writes a key, then its length unless it is key 10
fn enc_header((key, len): (u8, usize), out: &mut Vec<u8>) {
    encb::u8(key, out);
    if key != 0x10 {
        encb::u8_from_usize(len, out);
    }
}

/// Stops at a value that starts with DE AD: it needs two
/// bytes after the key to tell
fn stop_at_dead(_key: &u8, input: &mut &[u8]) -> BreakType {
    if input.starts_with(&[0xDE, 0xAD]) {
        return BreakType::Done;
    }
    // fewer than two bytes, a short read, also proceeds
    BreakType::Proceed
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(
        dec = decb::u8,
        enc = encb::u8,
    ),
    len(
        dec = decb::u8_as_usize,
        enc = encb::u8_from_usize,
    ),
    strict,
)]
/// A nested struct and a UTF-16 string, under `strict`
struct Outer {
    #[klv(
        key = 0x05,
        dec = Frame::decode_value,
        enc = Frame::encode_value,
    )]
    inner: Frame,

    #[klv(
        key = 0x06,
        dec = decs::to_string_utf16_be,
        enc = &encs::from_string_utf16_be,
        size(lengthed),
    )]
    name: Option<String>,
}

/// The kind of `decode_value`'s error, or `None` if it
/// decodes
fn error_kind<T>(wire: &[u8]) -> Option<ErrorKind>
where
    T: for<'a> DecodeValue<&'a [u8]>,
{
    T::decode_value(&mut &wire[..])
        .err()
        .and_then(|e| ErrorKind::of(&e))
}

fn main() {
    // a length that fails to read: BER's 80 ends the
    // decode, and the rest stays in the input
    let wire = [0x01, 0x01, 0x0A, 0x02, 0x80, 0x03, 0x01, 0x0C];
    let mut input = wire.as_slice();
    let three = Three::decode_value(&mut input).unwrap();
    println!("{wire:02X?} -> {three:?}");
    assert_eq!(
        three,
        Three {
            a: Some(10),
            b: None,
            c: None,
        }
    );
    assert_eq!(input, [0x02, 0x80, 0x03, 0x01, 0x0C]);

    // non-canonical BER decodes
    let padded: u64 = decber::ber_oid(&mut [0x80, 0x80, 0x05].as_slice()).unwrap();
    assert_eq!(padded, 5);
    let long_form = decber::ber_length(&mut [0x81, 0x05].as_slice());
    assert_eq!(long_form, Ok(5));

    // a u128 length of 2^64 + 5 reads as 5
    #[cfg(target_pointer_width = "64")]
    {
        let mut wire = Vec::new();
        encb::be_u128((1 << 64) + 5, &mut wire);
        assert_eq!(decb::be_u128_as_usize(&mut wire.as_slice()), Ok(5));
    }

    // two invalid bytes read as two U+FFFD: six bytes
    let text = decs::to_string_utf8(2)(&mut [0xFF, 0xFE].as_slice()).unwrap();
    assert_eq!(text, "\u{FFFD}\u{FFFD}");
    assert_eq!(text.len(), 6);

    // three packets, the second missing `a`
    #[rustfmt::skip]
    let wire = [
        0xAA, 0x03, 0x01, 0x01, 0x07, // a = 7
        0xAA, 0x03, 0x02, 0x01, 0x09, // b = 9, no a
        0xAA, 0x03, 0x01, 0x01, 0x08, // a = 8
    ];

    // drain_frames stops at the second, and returns the first
    let mut input = wire.as_slice();
    let frames = Frame::drain_frames(&mut input).unwrap();
    assert_eq!(frames, [Frame { a: 7, b: None }]);
    assert_eq!(input, &wire[5..]);

    // decode_frame there returns the error, and moves past
    // the second
    assert!(Frame::decode_frame(&mut input).is_err());
    assert_eq!(input, &wire[10..]);

    // Decoder::next returns None at the second, with the
    // third still buffered
    let mut decoder = Frame::decoder();
    decoder.feed(&wire);
    assert_eq!(decoder.next(), Some(Frame { a: 7, b: None }));
    assert_eq!(decoder.next::<Frame>(), None);
    assert_eq!(decoder.buffered(), &wire[10..]);
    assert_eq!(decoder.next(), Some(Frame { a: 8, b: None }));

    // next_fresh returns the second's error and moves on
    let mut decoder = Frame::decoder();
    decoder.feed(&wire);
    assert!(decoder.next_fresh().is_ok());
    let error = decoder.next_fresh().unwrap_err();
    assert_eq!(error.kind(), Some(ErrorKind::MissingRequired));
    assert_eq!(decoder.next_fresh().unwrap(), Frame { a: 8, b: None });

    // a packet length past 64 is reported once, then skipped
    let mut decoder = Frame::decoder();
    decoder.feed(&[0xAA, 0xFF, 0xAA, 0x03, 0x01, 0x01, 0x07]);
    let error = decoder.next_fresh().unwrap_err();
    assert_eq!(error.kind(), Some(ErrorKind::MalformedValue));
    assert_eq!(decoder.next_fresh().unwrap(), Frame { a: 7, b: None });

    // strict misses an item the end of a packet's body cuts
    let wire = [0xAA, 0x05, 0x01, 0x01, 0x07, 0x02, 0x02];
    let frame = Frame::decode_frame(&mut wire.as_slice());
    assert_eq!(frame.unwrap(), Frame { a: 7, b: None });

    // a streamed Vec pushes an element cut short
    let wire = [0x01, 0x01, 0x07, 0x02];
    let mut input = wire.as_slice();
    let three = Vec::<Three>::decode_partial(&mut input).unwrap();
    let Packet::Ready(three) = three else {
        panic!("the Vec is ready");
    };
    assert_eq!(
        three,
        [Three {
            a: Some(7),
            b: None,
            c: None,
        }]
    );
    assert_eq!(input, [0x02]);

    // a key literal leaves the end item's length: a Vec
    // reads it as its next element
    #[rustfmt::skip]
    let wire = [
        0x01, 0x01, 0x07, 0xFF, 0x00, // temp 7, end
        0x01, 0x01, 0x08, 0xFF, 0x00, // temp 8, end
    ];
    let readings = Vec::<Reading>::decode_value(&mut wire.as_slice());
    assert_eq!(
        readings.unwrap(),
        [Reading { temp: Some(7) }, Reading { temp: None }]
    );

    // Proceed on a short read, under keylen: key 10 names
    // no field and has no length, so the decode ends
    let mut input = [0x10, 0xAA].as_slice();
    let peeked = Peeked::decode_partial(&mut input).unwrap();
    assert!(matches!(peeked, Packet::Ready(Peeked { a: None })));
    assert_eq!(input, [0x10, 0xAA]);

    // under strict, a nested struct missing a field and an
    // odd-length UTF-16 value are both MalformedValue
    let nested = [0x05, 0x03, 0x02, 0x01, 0x09];
    assert_eq!(
        error_kind::<Outer>(&nested),
        Some(ErrorKind::MalformedValue)
    );
    #[rustfmt::skip]
    let odd = [
        0x05, 0x03, 0x01, 0x01, 0x07, // inner: a = 7
        0x06, 0x03, 0x00, 0x41, 0x00, // name: 3 bytes
    ];
    assert_eq!(error_kind::<Outer>(&odd), Some(ErrorKind::MalformedValue));
}
// ANCHOR_END: all
