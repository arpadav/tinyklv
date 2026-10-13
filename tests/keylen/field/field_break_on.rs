//! A field's `break_on`, a `fn(&value) -> BreakType` run right after that field's value decodes,
//! on a field of a fused `#[klv(keylen(..))]` struct
//!
//! It decides a value read from the input, framed `Inline`, as it decides one read from its
//! frame, framed `Sliced`: `Skip` discards the value, and `Done` keeps it and ends the packet
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::common::drive::Drive;
use crate::common::varint;
use crate::support::example::Status;
use crate::support::{header, value};
use tinyklv::prelude::*;

/// A journal whose error statuses are dropped and whose label is its last frame
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    keylen(dec = header::dec_tag),
    allow_unimplemented_encode,
)]
struct Journal {
    /// field 1, wire 0 VARINT - `Inline`
    #[klv(key = 0x08, dec = varint::dec_i32)]
    count: i32,

    /// field 2, wire 2 LEN - `Sliced(n)`, the journal's last frame
    #[klv(key = 0x12, dec = value::dec_string, break_on = Journal::ends)]
    label: Option<String>,

    /// field 3, wire 0 VARINT - `Inline` custom enum, an `Error` dropped
    #[klv(key = 0x18, dec = Status::decode_value, break_on = Journal::drops_errors)]
    status: Option<Status>,
}

/// [`Journal`] implementation
impl Journal {
    /// A label is its journal's last frame
    ///
    /// # Returns
    ///
    /// `Done`
    fn ends(_: &str) -> BreakType {
        BreakType::Done
    }

    /// An error status is dropped from its journal
    ///
    /// # Arguments
    ///
    /// * `status` - the decoded status
    ///
    /// # Returns
    ///
    /// `Skip` for an error, else `Proceed`
    fn drops_errors(status: &Status) -> BreakType {
        match status {
            Status::Error => BreakType::Skip,
            Status::Idle | Status::Active => BreakType::Proceed,
        }
    }
}

/// A field's `break_on` decides each value it decodes, in both drivers and at every chunk split:
/// `Skip` drops an `Inline` error status and keeps the status before it, and `Done` keeps the
/// `Sliced` label and leaves the frames after it unread
#[test]
fn a_fields_break_on_skips_or_ends_its_value() {
    #[rustfmt::skip]
    let journal = [
        // count = 3 (field 1, wire 0 VARINT, Inline)
        0x08, 0x03,
        // status = Active (field 3, wire 0 VARINT, Inline)
        0x18, 0x01,
        // status = Error (field 3, wire 0 VARINT, Inline) - skipped
        0x18, 0x02,
        // label = "hi" (field 2, wire 2 LEN, Sliced(2)) - ends the journal
        0x12, 0x02, 0x68, 0x69,
    ];
    // count = 9 (field 1, wire 0 VARINT, Inline) - never read
    let unread = [0x08, 0x09];
    let expected = Journal {
        count: 3,
        label: Some("hi".to_owned()),
        status: Some(Status::Active),
    };
    expected.assert_stops_at(&[&journal[..], &unread].concat(), journal.len());
}
