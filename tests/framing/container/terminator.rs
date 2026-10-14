//! A key + len struct ended by a key literal that is also one of its fields' keys: the terminator
//! the encode writes
//!
//! The literal ends the decode right after its key, so the encode writes that field after every
//! other frame, the ones the struct keeps for keys no field declares included, which the decode
//! then reads back
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::common::drive::Drive;
use crate::common::unknown::{self, Unknown};
use crate::common::varint;
use tinyklv::prelude::*;

/// A batch whose checksum, declared first, is its terminator, keeping the frames no field declares
#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(dec = varint::dec_u32, enc = varint::enc_u32),
    len(dec = varint::dec_usize, enc = varint::enc_usize),
    unknown(dec = unknown::unknown_dec, enc = unknown::unknown_enc),
    break_on = 0x1A,
)]
struct Batch {
    /// key 0x1A, never decoded: its key ends the decode
    #[klv(key = 0x1A, dec = varint::dec_u32, enc = *varint::enc_u32)]
    checksum: Option<u32>,

    /// key 0x38
    #[klv(key = 0x38, dec = varint::dec_u32, enc = *varint::enc_u32)]
    count: Option<u32>,

    /// every other frame, keyed by its tag
    #[klv(unknown)]
    extra: Vec<(u32, Unknown)>,
}

/// The terminator is encoded after the count and the kept frame, though declared first, so the
/// decode its key ends has read them both
#[test]
fn a_terminator_field_is_encoded_after_the_kept_frames() {
    let batch = Batch {
        checksum: Some(0x2A),
        count: Some(13),
        extra: vec![(0x40, Unknown::Varint(5))],
    };
    #[rustfmt::skip]
    let wire = [
        // count = 13
        0x38, 0x01, 0x0D,
        // a kept frame: tag 0x40, varint 5
        0x40, 0x05,
        // the terminator: its key, then its length and value, never read
        0x1A, 0x01, 0x2A,
    ];
    assert_eq!(batch.encoded(), wire);
    let read = Batch {
        checksum: None,
        ..batch
    };
    read.assert_stops_at(&wire, 6);
}
