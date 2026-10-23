//! A struct's `break_on`, checked right after each frame's key, before its length
//!
//! A key literal ends the decode with the input right after the key. A fn is handed the key and
//! the input after it, and may read on: `Done` ends the decode where the fn left the input, and
//! `Proceed` rewinds to just after the key, where the frame decodes as if the fn read nothing
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::common::drive::Drive;
use tinyklv::dec::binary as decb;
use tinyklv::prelude::*;

// --------------------------------------------------
// external
// --------------------------------------------------
use winnow::binary::length_take;

/// A temperature frame, the marker frame, then a temperature frame the marker comes before
#[rustfmt::skip]
const WIRE: &[u8] = &[
    // temp = 7
    0x01, 0x01, 0x07,
    // the marker: key 0xFF, length 1, its value
    0xFF, 0x01, 0x02,
    // temp = 8
    0x01, 0x01, 0x08,
];

/// A reading ended by the marker key
#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(dec = decb::u8),
    len(dec = decb::u8_as_usize),
    allow_unimplemented_encode,
    break_on = 0xFF,
)]
struct Thermometer {
    /// key 0x01
    #[klv(key = 0x01, dec = decb::u8)]
    temp: u8,
}

/// A reading ended after the marker frame, which its fn reads to the end
#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(dec = decb::u8),
    len(dec = decb::u8_as_usize),
    allow_unimplemented_encode,
    break_on = Marked::end_after_marker,
)]
struct Marked {
    /// key 0x01
    #[klv(key = 0x01, dec = decb::u8)]
    temp: u8,
}

/// [`Marked`] implementation
impl Marked {
    /// Ends the reading once the marker's length and value are read
    ///
    /// # Arguments
    ///
    /// * `key` - the frame's key
    /// * `input` - the input right after the key
    ///
    /// # Returns
    ///
    /// `Done` past the marker frame, else `Proceed`, a marker cut short included, which then
    /// decodes as a frame cut short
    fn end_after_marker(key: &u8, input: &mut &[u8]) -> BreakType {
        if *key != 0xFF {
            return BreakType::Proceed;
        }
        match length_take(decb::u8_as_usize).parse_next(input) {
            Ok(_) => BreakType::Done,
            Err(_) => BreakType::Proceed,
        }
    }
}

/// A reading whose fn reads every frame, then lets it decode
#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(dec = decb::u8),
    len(dec = decb::u8_as_usize),
    allow_unimplemented_encode,
    break_on = Rewound::read_then_proceed,
)]
struct Rewound {
    /// key 0x01
    #[klv(key = 0x01, dec = decb::u8)]
    temp: u8,
}

/// [`Rewound`] implementation
impl Rewound {
    /// Reads the frame's length and value, as far as the input holds them
    ///
    /// # Arguments
    ///
    /// * `_` - the frame's key
    /// * `input` - the input right after the key
    ///
    /// # Returns
    ///
    /// `Proceed`
    fn read_then_proceed(_: &u8, input: &mut &[u8]) -> BreakType {
        let _ = length_take(decb::u8_as_usize).parse_next(input);
        BreakType::Proceed
    }
}

/// A reading padded with key `0x7E`, which its key decoder peeks and its fn skips
#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(dec = Padding::peek_padding),
    len(dec = decb::u8_as_usize),
    allow_unimplemented_encode,
    break_on = Padding::skip_padding,
)]
struct Padding {
    /// key 0x01
    #[klv(key = 0x01, dec = decb::u8)]
    temp: Option<u8>,
}

/// [`Padding`] implementation
impl Padding {
    /// Decodes a key, peeking the padding key `0x7E` without consuming it
    ///
    /// # Arguments
    ///
    /// * `input` - the input positioned at a key; left in place at the padding key
    ///
    /// # Returns
    ///
    /// The key, or the error of [`decb::u8`]
    fn peek_padding(input: &mut &[u8]) -> tinyklv::Result<u8> {
        match input.first() {
            Some(&0x7E) => Ok(0x7E),
            _ => decb::u8(input),
        }
    }

    /// Skips the padding, reading nothing of it
    ///
    /// # Arguments
    ///
    /// * `key` - the frame's key
    /// * `_input` - the input right after the key, never read
    ///
    /// # Returns
    ///
    /// `Skip` for padding, else `Proceed`
    fn skip_padding(key: &u8, _input: &mut &[u8]) -> BreakType {
        if *key == 0x7E {
            BreakType::Skip
        } else {
            BreakType::Proceed
        }
    }
}

/// A `Skip` of a frame that consumed nothing could only repeat it: both drivers reject it, rather
/// than loop, which the check, run on its own thread, is given a second to show
#[test]
fn a_skip_that_consumed_nothing_is_rejected() {
    #[rustfmt::skip]
    const PADDED: &[u8] = &[
        // temp = 7
        0x01, 0x01, 0x07,
        // padding, its key never consumed
        0x7E,
    ];
    let (done, finished) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        Padding::assert_rejects_naming(PADDED, &["`Padding` item", "consumed no input"]);
        let _ = done.send(());
    });
    assert_eq!(
        finished.recv_timeout(std::time::Duration::from_secs(1)),
        Ok(()),
        "the check panicked or never returned"
    );
}

/// A key literal ends the decode right after the marker's key, its length and value unread
#[test]
fn a_key_literal_stops_right_after_the_key() {
    Thermometer { temp: 7 }.assert_stops_at(WIRE, 4);
}

/// A fn ends the decode where it leaves the input, past the marker's length and value
#[test]
fn a_fn_stops_where_it_leaves_the_input() {
    Marked { temp: 7 }.assert_stops_at(WIRE, 6);
}

/// `Proceed` rewinds what the fn read, so every frame decodes as if the fn read nothing
#[test]
fn proceed_rewinds_to_just_after_the_key() {
    Rewound { temp: 8 }.assert_decodes_from(WIRE);
}
