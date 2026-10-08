//! A field's `break_on`, a `fn(&value) -> BreakType` run right after that field's value decodes
//!
//! `Skip` discards the value, `Done` keeps it and ends the packet, and `Abort` rejects it, naming
//! the field. It runs on a value read straight from the input and on one sliced by its length
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::common::drive::Drive;
use crate::common::varint;
use tinyklv::dec::binary as decb;
use tinyklv::prelude::*;

/// A sensor's health, a varint on the wire
#[derive(Debug, Clone, Copy, PartialEq)]
enum Health {
    /// `0`, kept
    Nominal,

    /// `1`, dropped from the report
    Degraded,

    /// `2`, rejects the report
    Corrupt,
}

/// [`Health`] implementation
impl Health {
    /// Decodes a health code
    ///
    /// # Arguments
    ///
    /// * `input` - the input, at the code's varint
    ///
    /// # Returns
    ///
    /// The health the code names
    fn decode(input: &mut &[u8]) -> tinyklv::Result<Self> {
        varint::dec_u32
            .verify_map(|code| match code {
                0 => Some(Self::Nominal),
                1 => Some(Self::Degraded),
                2 => Some(Self::Corrupt),
                _ => None,
            })
            .parse_next(input)
    }

    /// What a decoded health does to its report
    ///
    /// # Returns
    ///
    /// `Proceed` for nominal, `Skip` for degraded, `Abort` for corrupt
    fn verdict(&self) -> BreakType {
        match self {
            Self::Nominal => BreakType::Proceed,
            Self::Degraded => BreakType::Skip,
            Self::Corrupt => BreakType::Abort("sensor reported corrupt"),
        }
    }
}

/// A report's checksum, two big-endian bytes
#[derive(Debug, Clone, Copy, PartialEq)]
struct Checksum(u16);

/// [`Checksum`] implementation
impl Checksum {
    /// Decodes a checksum
    ///
    /// # Arguments
    ///
    /// * `input` - the frame's two value bytes
    ///
    /// # Returns
    ///
    /// The checksum
    fn decode(input: &mut &[u8]) -> tinyklv::Result<Self> {
        decb::be_u16.map(Self).parse_next(input)
    }

    /// A checksum is its report's last frame
    ///
    /// # Returns
    ///
    /// `Done`
    fn ends_report(_: &Self) -> BreakType {
        BreakType::Done
    }
}

/// A sensor's report, its health read straight from the input and its other values sliced by
/// their lengths
#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(dec = varint::dec_u32),
    len(dec = varint::dec_usize),
    allow_unimplemented_encode,
)]
struct Report {
    /// field 1, wire type 0
    #[klv(key = 0x08, dec = Health::decode, size(implicit), break_on = Health::verdict)]
    health: Option<Health>,

    /// field 2, wire type 2
    #[klv(key = 0x12, dec = varint::dec_u32)]
    seq: u32,

    /// field 3, wire type 2
    #[klv(
        key = 0x1A,
        dec = Checksum::decode,
        size(exact = 2),
        break_on = Checksum::ends_report,
    )]
    checksum: Option<Checksum>,
}

/// [`Report`] implementation
impl Report {
    /// health: nominal
    const NOMINAL: &[u8] = &[0x08, 0x00];

    /// health: degraded
    const DEGRADED: &[u8] = &[0x08, 0x01];

    /// health: corrupt
    const CORRUPT: &[u8] = &[0x08, 0x02];

    /// seq: 7
    const SEQ: &[u8] = &[0x12, 0x01, 0x07];

    /// checksum: `0xBEEF`
    const CHECKSUM: &[u8] = &[0x1A, 0x02, 0xBE, 0xEF];

    /// A nominal report, `seq` 7, before its checksum
    fn nominal() -> Self {
        Self {
            health: Some(Health::Nominal),
            seq: 7,
            checksum: None,
        }
    }
}

/// A field's `break_on` decides each value it decodes, in both drivers and at every chunk split:
/// `Skip` drops a degraded health and keeps the one before it, `Done` keeps the checksum and
/// leaves the frames after it unread, and `Abort` rejects a corrupt health naming the field, then
/// (one-shot) the fn's message
#[test]
fn a_fields_break_on_skips_ends_or_rejects_its_value() {
    // --------------------------------------------------
    // Skip
    // --------------------------------------------------
    let skipped = [Report::NOMINAL, Report::SEQ, Report::DEGRADED].concat();
    Report::nominal().assert_decodes_from(&skipped);
    // --------------------------------------------------
    // Done
    // --------------------------------------------------
    let ended = [
        Report::NOMINAL,
        Report::SEQ,
        Report::CHECKSUM,
        Report::CORRUPT,
    ]
    .concat();
    let with_checksum = Report {
        checksum: Some(Checksum(0xBEEF)),
        ..Report::nominal()
    };
    with_checksum.assert_stops_at(&ended, ended.len() - Report::CORRUPT.len());
    // --------------------------------------------------
    // Abort
    // --------------------------------------------------
    let rejected = [Report::NOMINAL, Report::CORRUPT].concat();
    Report::assert_rejects_naming(&rejected, &["`Report::health` value", "returned `Abort`"]);
    Report::assert_oneshot_label_chain(&rejected, "Report::health", "sensor reported corrupt");
}
