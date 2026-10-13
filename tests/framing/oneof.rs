//! `key = oneof(K => Enum::Variant(Type), ..)` on an `Option<Enum>` field of a key + len struct:
//! several keys route to one field
//!
//! Each arm's value is framed and coded by its type's `default(typ = ..)`, so a `size(implicit)`
//! arm and a prefixed one mix in one oneof, and an arm's frame must be as long as its type's
//! `exact = N`. Arms of one type are told apart by their keys alone. Each arm's key is a declared
//! key, to `deny_unknown_keys` and beside `unknown(..)` alike. The last arm decoded wins,
//! whichever arm came before it. Encode writes the one variant that is set, with its arm's key and
//! framing, at the field's place among the others; `None` writes nothing. An arm's frame cut short
//! at the end of the input is truncation: streaming pauses at the frame's start and completes once
//! the rest arrives
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::common::drive::Drive;
use crate::common::frames;
use crate::common::unknown::{self, Unknown};
use crate::common::varint;
use tinyklv::dec::string as decs;
use tinyklv::enc::string as encs;
use tinyklv::prelude::*;
use tinyklv::traits::ResumePartial;

/// A reading's payload: a count or a note
#[derive(Debug, Clone, PartialEq)]
enum Payload {
    /// field 7, wire type 0
    Count(i32),

    /// field 8, wire type 2
    Note(String),
}

/// A proto3 message in key + len mode with a oneof between two fields: `int32 id = 1`,
/// `oneof payload { int32 count = 7; string note = 8; }` and `string unit = 9`
#[derive(Klv, Debug, Clone, PartialEq)]
#[klv(
    stream = &[u8], // default, shown for clarity
    key(dec = varint::dec_u32, enc = varint::enc_u32),
    len(dec = varint::dec_usize, enc = varint::enc_usize),
    default(typ = i32, dec = varint::dec_i32, enc = *varint::enc_i32, size(implicit)),
    default(
        typ = String,
        dec = decs::to_string_utf8,
        enc = &encs::from_string_utf8,
        size(lengthed),
    ),
)]
struct Reading {
    /// field 1, wire type 0
    #[klv(key = 0x08)]
    id: i32,

    /// fields 7 and 8: a count, wire type 0, or a note, wire type 2
    #[klv(key = oneof(0x38 => Payload::Count(i32), 0x42 => Payload::Note(String)))]
    payload: Option<Payload>,

    /// field 9, wire type 2; optional, so `None` writes no frame
    #[klv(key = 0x4A)]
    unit: Option<String>,
}

/// [`Reading`] implementation
impl Reading {
    /// count = 300: tag, varint `AC 02`, no length
    const COUNT: &[u8] = &[0x38, 0xAC, 0x02];

    /// note = "hi": tag, length 2, "hi"
    const NOTE: &[u8] = &[0x42, 0x02, b'h', b'i'];

    /// unit = "mm": tag, length 2, "mm"
    const UNIT: &[u8] = &[0x4A, 0x02, b'm', b'm'];

    /// The reading [`frames::ID`] then [`Reading::COUNT`] hold
    fn counted() -> Self {
        Self {
            id: 5,
            payload: Some(Payload::Count(300)),
            unit: None,
        }
    }

    /// The reading [`frames::ID`] then [`Reading::NOTE`] hold
    fn noted() -> Self {
        Self {
            payload: Some(Payload::Note("hi".to_owned())),
            ..Self::counted()
        }
    }

    /// Each arm's frame, and the reading it holds after [`frames::ID`]
    fn arms() -> [(&'static [u8], Self); 2] {
        [(Self::COUNT, Self::counted()), (Self::NOTE, Self::noted())]
    }
}

/// [`Reading`] under `deny_unknown_keys`, without its unit
#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(dec = varint::dec_u32),
    len(dec = varint::dec_usize),
    allow_unimplemented_encode,
    deny_unknown_keys,
    default(typ = i32, dec = varint::dec_i32, size(implicit)),
    default(typ = String, dec = decs::to_string_utf8, size(lengthed)),
)]
struct StrictReading {
    /// field 1, wire type 0
    #[klv(key = 0x08)]
    id: i32,

    /// fields 7 and 8: a count, wire type 0, or a note, wire type 2
    #[klv(key = oneof(0x38 => Payload::Count(i32), 0x42 => Payload::Note(String)))]
    payload: Option<Payload>,
}

/// [`Reading`] whose note is a two-letter code, `exact = 2` from its type's `default(..)`
#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(dec = varint::dec_u32),
    len(dec = varint::dec_usize),
    allow_unimplemented_encode,
    default(typ = i32, dec = varint::dec_i32, size(implicit)),
    default(typ = String, dec = decs::to_string_utf8, size(lengthed, exact = 2)),
)]
struct CodedReading {
    /// field 1, wire type 0
    #[klv(key = 0x08)]
    id: i32,

    /// fields 7 and 8: a count, wire type 0, or a two-letter note, wire type 2
    #[klv(key = oneof(0x38 => Payload::Count(i32), 0x42 => Payload::Note(String)))]
    payload: Option<Payload>,
}

/// [`Reading`] keeping the frames neither a field nor an arm declares, without its unit
#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(dec = varint::dec_u32, enc = varint::enc_u32),
    len(dec = varint::dec_usize, enc = varint::enc_usize),
    unknown(dec = unknown::unknown_dec, enc = unknown::unknown_enc),
    default(typ = i32, dec = varint::dec_i32, enc = *varint::enc_i32, size(implicit)),
    default(
        typ = String,
        dec = decs::to_string_utf8,
        enc = &encs::from_string_utf8,
        size(lengthed),
    ),
)]
struct KeptReading {
    /// field 1, wire type 0
    #[klv(key = 0x08)]
    id: i32,

    /// fields 7 and 8: a count, wire type 0, or a note, wire type 2
    #[klv(key = oneof(0x38 => Payload::Count(i32), 0x42 => Payload::Note(String)))]
    payload: Option<Payload>,

    /// every other frame, keyed by its tag
    #[klv(unknown)]
    extra: Vec<(u32, Unknown)>,
}

/// A gauge's level: low or high, an `i32` on the wire either way
#[derive(Debug, PartialEq)]
enum Level {
    /// field 1, wire type 0
    Low(i32),

    /// field 2, wire type 0
    High(i32),
}

/// A gauge whose oneof arms share one type, so only the key tells them apart: `oneof level {
/// int32 low = 1; int32 high = 2; }`
#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(dec = varint::dec_u32, enc = varint::enc_u32),
    len(dec = varint::dec_usize, enc = varint::enc_usize),
    default(typ = i32, dec = varint::dec_i32, enc = *varint::enc_i32, size(implicit)),
)]
struct Gauge {
    /// fields 1 and 2: a low or a high level, both wire type 0
    #[klv(key = oneof(0x08 => Level::Low(i32), 0x10 => Level::High(i32)))]
    level: Option<Level>,
}

/// The last arm on the wire wins, in both drivers and at every chunk split: a note after a count,
/// a count after a note, and the second of two notes
#[test]
fn the_last_decoded_arm_wins() {
    // note = "bye": tag, length 3, "bye"
    let bye: &[u8] = &[0x42, 0x03, b'b', b'y', b'e'];
    let rewritten = Reading {
        payload: Some(Payload::Note("bye".to_owned())),
        ..Reading::noted()
    };
    // --------------------------------------------------
    // (the first arm's frame, the last arm's frame, what the wire decodes to)
    // --------------------------------------------------
    let cases = [
        (Reading::COUNT, Reading::NOTE, Reading::noted()),
        (Reading::NOTE, Reading::COUNT, Reading::counted()),
        (Reading::NOTE, bye, rewritten),
    ];
    for (first, last, decoded) in cases {
        decoded.assert_decodes_from(&[frames::ID, first, last].concat());
    }
}

/// Each arm takes its type's framing: the `size(implicit)` count has no length after its key,
/// the note has one, and each decodes from and encodes to exactly its frame
#[test]
fn each_arm_takes_its_types_framing() {
    for (frame, reading) in Reading::arms() {
        let wire = [frames::ID, frame].concat();
        assert_eq!(reading.encoded(), wire);
        reading.assert_decodes_from(&wire);
    }
}

/// `None` writes no frame, so the fields around the oneof are written back to back, and a wire
/// with no arm decodes to `None`; `Some` writes its arm's frame between them
#[test]
fn an_unset_oneof_writes_nothing_between_the_fields_around_it() {
    let set = Reading {
        unit: Some("mm".to_owned()),
        ..Reading::counted()
    };
    let wire = [frames::ID, Reading::COUNT, Reading::UNIT].concat();
    assert_eq!(set.encoded(), wire);
    set.assert_decodes_from(&wire);
    let unset = Reading {
        payload: None,
        ..set
    };
    let wire = [frames::ID, Reading::UNIT].concat();
    assert_eq!(unset.encoded(), wire);
    unset.assert_decodes_from(&wire);
}

/// An arm's frame cut inside its value, the implicit count's or the prefixed note's, is
/// truncation: streaming pauses with the frame unconsumed, and resumed with the rest it decodes
/// to what one-shot decodes the whole wire to
#[test]
fn a_oneof_frame_cut_across_two_chunks_resumes() {
    for (frame, whole) in Reading::arms() {
        let wire = [frames::ID, frame].concat();
        let (sent, last) = wire.split_at(wire.len() - 1);
        let held = frame.len() - 1;
        assert_eq!(Reading::decode_value(&mut &wire[..]), Ok(whole.clone()));
        // --------------------------------------------------
        // the first chunk: pauses at the start of the cut frame
        // --------------------------------------------------
        let mut cursor = sent;
        let Ok(Packet::NeedMore(partial)) = Reading::decode_partial(&mut cursor) else {
            panic!("expected NeedMore for {sent:02X?}");
        };
        assert_eq!(cursor.len(), held, "streaming paused at the cut frame");
        // --------------------------------------------------
        // the second chunk, after the unconsumed bytes: completes the frame
        // --------------------------------------------------
        let resumed = [cursor, last].concat();
        match Reading::resume_partial(&mut &resumed[..], partial) {
            Ok(Packet::NeedMore(partial)) => assert_eq!(partial.finalize(), Ok(whole)),
            other => panic!("expected NeedMore at the end of {wire:02X?}, got {other:?}"),
        }
    }
}

/// Arms of one type keep their own keys: each variant encodes to, and decodes from in both drivers
/// and at every chunk split, the frame of the arm that names it
#[test]
fn arms_of_one_type_keep_their_own_keys() {
    // --------------------------------------------------
    // (the frame, the level it holds)
    // --------------------------------------------------
    let cases: [(&[u8], _); 2] = [
        // low = 5: tag 1, varint 5
        (&[0x08, 0x05], Level::Low(5)),
        // high = 5: tag 2, varint 5
        (&[0x10, 0x05], Level::High(5)),
    ];
    for (frame, level) in cases {
        let gauge = Gauge { level: Some(level) };
        assert_eq!(gauge.encoded(), frame);
        gauge.assert_decodes_from(frame);
    }
}

/// Under `deny_unknown_keys` each arm's key is a declared one, so its frame decodes in both
/// drivers and at every chunk split, while a key neither a field nor an arm declares is rejected
#[test]
fn deny_unknown_keys_accepts_each_arms_key() {
    for (frame, reading) in Reading::arms() {
        let strict = StrictReading {
            id: reading.id,
            payload: reading.payload,
        };
        strict.assert_decodes_from(&[frames::ID, frame].concat());
    }
    let unknown = [frames::ID, frames::UNKNOWN_VARINT].concat();
    StrictReading::assert_rejects_naming(&unknown, &["`StrictReading`", "`deny_unknown_keys`"]);
}

/// An arm takes the `exact = N` of its type's `default(..)`: a frame `N` bytes long decodes, and
/// one of another length is rejected in both drivers, naming the field and the width, with the
/// cursor past the frame
#[test]
fn an_arm_takes_its_types_exact_width() {
    let coded = CodedReading {
        id: 5,
        payload: Some(Payload::Note("hi".to_owned())),
    };
    coded.assert_decodes_from(&[frames::ID, Reading::NOTE].concat());
    let wire = [
        frames::ID,
        // note = "bye": tag, length 3, "bye"
        &[0x42, 0x03, b'b', b'y', b'e'],
    ]
    .concat();
    let message =
        "`CodedReading::payload` value (its frame length does not match `size(exact = 2)`)";
    CodedReading::assert_rejects_at(&wire, wire.len(), message);
}

/// Beside `unknown(..)` each arm's key still reaches the oneof, and a key no arm declares reaches
/// the unknown decoder, in both drivers and at every chunk split; encode writes the arm at the
/// field's place and the kept frame after the known fields
#[test]
fn a_oneof_beside_unknown_keeps_its_arms() {
    let kept = KeptReading {
        id: 5,
        payload: Some(Payload::Note("hi".to_owned())),
        extra: vec![frames::varint_entry()],
    };
    kept.assert_decodes_from(&[frames::ID, frames::UNKNOWN_VARINT, Reading::NOTE].concat());
    let encoded = [frames::ID, Reading::NOTE, frames::UNKNOWN_VARINT].concat();
    assert_eq!(kept.encoded(), encoded);
}
