//! A sentinel struct with a lifetime parameter named `'z`
//!
//! The `SeekSentinel` impl's fns and the `decoder()` constructor declare a lifetime of their own
//! inside impls generic over the struct's, so one named like the struct's would shadow it
//! (E0496): [`Tagged`] must decode from its sentinel frame, one-shot and through its `Decoder`
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use tinyklv::dec::binary as decb;
use tinyklv::enc::binary as encb;
use tinyklv::prelude::*;

// --------------------------------------------------
// external
// --------------------------------------------------
use std::marker::PhantomData;

/// A level behind a sentinel, generic over a lifetime its plain field marks
#[derive(Klv, Debug, PartialEq)]
#[klv(
    sentinel = b"\xAA",
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct Tagged<'z> {
    /// key `0x01`, one byte
    #[klv(key = 0x01, dec = decb::u8, enc = *encb::u8)]
    level: u8,

    /// no key, the lifetime's marker
    marker: PhantomData<&'z ()>,
}

/// [`Tagged`] decodes from its frame one-shot and through its `Decoder`
#[test]
fn a_lifetime_named_z_decodes() {
    let tagged = || Tagged {
        level: 5,
        marker: PhantomData,
    };
    let frame = [0xAA, 0x03, 0x01, 0x01, 0x05];
    assert_eq!(Tagged::decode_frame(&mut &frame[..]), Ok(tagged()));
    let mut decoder = Tagged::decoder();
    decoder.feed(&frame);
    assert_eq!(decoder.next(), Some(tagged()));
}
