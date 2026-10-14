//! A field's `break_on`, a `fn(&value) -> BreakType` run right after that field's value decodes
//!
//! `Skip` discards the value, `Done` keeps it and ends the packet, and `Abort` rejects it with the
//! fn's message. It runs on a value read straight from the input and on one sliced by its length,
//! and after an earlier field whose decoder's path names a `for<'a>` type. Encode writes a field
//! with a `break_on` after every other frame
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::common::drive::Drive;
use crate::common::unknown::{self, Unknown};
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

/// A check a decoded byte must pass, implemented by the fn-pointer type that names it
trait Check {
    /// Whether `byte` passes
    fn check(byte: &u8) -> bool;
}

/// [`Check`] implementation for a higher-ranked fn pointer: every byte but `0` passes
impl Check for for<'a> fn(&'a u8) {
    fn check(byte: &u8) -> bool {
        *byte != 0
    }
}

/// Decodes one byte that `F` passes
///
/// # Arguments
///
/// * `input` - the frame's value byte
///
/// # Returns
///
/// The byte
fn checked<F: Check>(input: &mut &[u8]) -> tinyklv::Result<u8> {
    decb::u8.verify(F::check).parse_next(input)
}

/// A reading whose level decoder's path names a higher-ranked `for<'a>` type, declared before a
/// field with a `break_on`
#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(dec = decb::u8),
    len(dec = decb::u8_as_usize),
    allow_unimplemented_encode,
)]
struct Ranked {
    /// key `0x01`, one byte
    #[klv(key = 0x01, dec = checked::<for<'a> fn(&'a u8)>)]
    level: Option<u8>,

    /// key `0x02`, the reading's last frame
    #[klv(key = 0x02, dec = Checksum::decode, break_on = Checksum::ends_report)]
    checksum: Option<Checksum>,
}

/// A log entry's status, a varint on the wire
#[derive(Debug, Clone, Copy, PartialEq)]
enum Status {
    /// `0`
    Clean,

    /// `1`, which ends the log
    Corrupt,
}

/// [`Status`] implementation
impl Status {
    /// Decodes a status code
    ///
    /// # Arguments
    ///
    /// * `input` - the input, at the code's varint
    ///
    /// # Returns
    ///
    /// The status the code names
    fn decode(input: &mut &[u8]) -> tinyklv::Result<Self> {
        varint::dec_u32
            .verify_map(|code| match code {
                0 => Some(Self::Clean),
                1 => Some(Self::Corrupt),
                _ => None,
            })
            .parse_next(input)
    }

    /// Encodes the status code
    ///
    /// # Arguments
    ///
    /// * `out` - the buffer the code's varint is appended to
    fn encode(&self, out: &mut Vec<u8>) {
        let code = match self {
            Self::Clean => 0,
            Self::Corrupt => 1,
        };
        varint::enc_u32(code, out);
    }

    /// What a decoded status does to its log
    ///
    /// # Returns
    ///
    /// `Proceed` for clean, `Done` for corrupt
    fn ends_log(&self) -> BreakType {
        match self {
            Self::Clean => BreakType::Proceed,
            Self::Corrupt => BreakType::Done,
        }
    }
}

/// A log entry whose status, declared first, ends the decode when corrupt, keeping the frames no
/// field declares
#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(dec = varint::dec_u32, enc = varint::enc_u32),
    len(dec = varint::dec_usize, enc = varint::enc_usize),
    unknown(dec = unknown::unknown_dec, enc = unknown::unknown_enc),
)]
struct Log {
    /// key 0x1A
    #[klv(
        key = 0x1A,
        dec = Status::decode,
        enc = Status::encode,
        break_on = Status::ends_log,
    )]
    status: Status,

    /// key 0x38
    #[klv(key = 0x38, dec = varint::dec_u32, enc = *varint::enc_u32)]
    count: u32,

    /// every other frame, keyed by its tag
    #[klv(unknown)]
    extra: Vec<(u32, Unknown)>,
}

/// A field with a `break_on` is encoded after every other frame, though declared first, so the
/// decode its `Done` ends has read them all: a corrupt log round-trips with its count and the
/// frame it kept
#[test]
fn a_field_with_a_break_on_is_encoded_last() {
    let log = Log {
        status: Status::Corrupt,
        count: 13,
        extra: vec![(0x40, Unknown::Varint(5))],
    };
    #[rustfmt::skip]
    let wire = [
        // count = 13
        0x38, 0x01, 0x0D,
        // a kept frame: tag 0x40, varint 5
        0x40, 0x05,
        // status = corrupt, which ends the decode
        0x1A, 0x01, 0x01,
    ];
    assert_eq!(log.encoded(), wire);
    log.assert_stops_at(&wire, wire.len());
}

/// A field's `break_on` decides each value it decodes, in both drivers and at every chunk split:
/// `Skip` drops a degraded health and keeps the one before it, `Done` keeps the checksum and
/// leaves the frames after it unread, and `Abort` rejects a corrupt health with the fn's message,
/// after (one-shot) a label naming the field
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
    for at in 0..=rejected.len() {
        let (outcome, _) = Report::stream_split(&rejected, at);
        assert!(
            matches!(outcome, Err(err) if err.label() == "sensor reported corrupt"),
            "split at {at}: {outcome:?}",
        );
    }
    Report::assert_oneshot_label_chain(&rejected, "Report::health", "sensor reported corrupt");
}

/// A `for<'a>` in a field decoder's path opens no loop of the derive's: the checksum's `Done`
/// after it still ends the reading, in both drivers, with the frame after it unread
#[test]
fn a_higher_ranked_decoder_path_keeps_a_later_break_on() {
    let ranked = Ranked {
        level: Some(5),
        checksum: Some(Checksum(0xBEEF)),
    };
    #[rustfmt::skip]
    let wire = [
        // level = 5
        0x01, 0x01, 0x05,
        // checksum = 0xBEEF, which ends the reading
        0x02, 0x02, 0xBE, 0xEF,
        // level = 6, never read
        0x01, 0x01, 0x06,
    ];
    ranked.assert_stops_at(&wire, 7);
}
