//! `skip_if_default` and `skip_if = <fn>`: a field's frame is left out of the encode
//!
//! A field skipped at its type's default, from its `default(typ = ..)`, or by its own predicate
//! writes no key, length or value, and a missing frame decodes to the type's default. A field's
//! own skip replaces its default's, and its own `default` is what a missing frame decodes to: on an
//! `Option` field, a bare `default` is `None`
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::common::drive::Drive;
use crate::common::frames;
use crate::common::varint;
use tinyklv::dec::binary as decb;
use tinyklv::dec::string as decs;
use tinyklv::enc::binary as encb;
use tinyklv::enc::string as encs;
use tinyklv::prelude::*;

/// The plan's worked example, one proto3 message in key + len mode: `int32 id = 1` (a varint),
/// `float ratio = 2` (four bytes), `string name = 3` (length-delimited) and `fixed64
/// created_at = 4` (eight bytes), each left out at its default
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8], // default, shown for clarity
    key(dec = varint::dec_u32, enc = varint::enc_u32),
    len(dec = varint::dec_usize, enc = varint::enc_usize),
    default(
        typ = i32,
        dec = varint::dec_i32,
        enc = *varint::enc_i32,
        size(implicit),
        skip_if_default,
    ),
    default(
        typ = f32,
        dec = decb::le_f32,
        enc = *encb::le_f32,
        size(implicit, exact = 4),
        skip_if_default,
    ),
    default(
        typ = u64,
        dec = decb::le_u64,
        enc = *encb::le_u64,
        size(implicit, exact = 8),
        skip_if_default,
    ),
)]
struct Reading {
    /// field 1, wire type 0
    #[klv(key = 0x08)]
    id: i32,

    /// field 2, wire type 5
    #[klv(key = 0x15)]
    ratio: f32,

    /// field 3, wire type 2
    #[klv(
        key = 0x1A,
        dec = decs::to_string_utf8,
        enc = &encs::from_string_utf8,
        size(lengthed),
        skip_if = String::is_empty,
    )]
    name: String,

    /// field 4, wire type 1
    #[klv(key = 0x21)]
    created_at: u64,
}

/// [`Reading`] implementation
impl Reading {
    /// The plan's value, `Reading { id: 5, ratio: 1.5, name: "hi", created_at: 0 }`
    fn decoded() -> Self {
        Self {
            id: 5,
            ratio: 1.5,
            name: "hi".to_owned(),
            created_at: 0,
        }
    }
}

/// A thermostat whose every `i32` is left out at 0, but whose setpoint is left out at room
/// temperature, which a missing setpoint decodes to
#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(dec = varint::dec_u32, enc = varint::enc_u32),
    len(dec = varint::dec_usize, enc = varint::enc_usize),
    default(
        typ = i32,
        dec = varint::dec_i32,
        enc = *varint::enc_i32,
        size(implicit),
        skip_if_default,
    ),
)]
struct Thermostat {
    /// field 1, left out at 0
    #[klv(key = 0x08)]
    offset: i32,

    /// field 2, left out at room temperature
    #[klv(key = 0x10, skip_if = Thermostat::at_room, default = Thermostat::ROOM)]
    setpoint: i32,
}

/// [`Thermostat`] implementation
impl Thermostat {
    /// Room temperature, in degrees Celsius
    const ROOM: i32 = 20;

    /// Whether a setpoint is room temperature
    ///
    /// # Arguments
    ///
    /// * `setpoint` - the setpoint
    fn at_room(setpoint: &i32) -> bool {
        *setpoint == Self::ROOM
    }
}

/// A heater whose optional schedule is left out at its default, `None`
#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(dec = varint::dec_u32, enc = varint::enc_u32),
    len(dec = varint::dec_usize, enc = varint::enc_usize),
)]
struct Heater {
    /// field 1, a schedule id
    #[klv(
        key = 0x08,
        dec = varint::dec_i32,
        enc = *varint::enc_i32,
        size(implicit),
        default,
        skip_if_default,
    )]
    schedule: Option<i32>,
}

/// The worked example with `created_at` at its default encodes to exactly the plan's bytes, no
/// frame for `created_at`, and decodes back from them
#[test]
fn the_worked_example_round_trips_to_the_plans_bytes() {
    let wire = [frames::ID, frames::RATIO, frames::NAME].concat();
    assert_eq!(Reading::decoded().encoded(), wire);
    Reading::decoded().assert_decodes_from(&wire);
}

/// A `skip_if` predicate that holds leaves the field's frame out, and the missing frame decodes
/// to the type's default
#[test]
fn a_skip_if_predicate_leaves_its_frame_out() {
    let unnamed = Reading {
        name: String::new(),
        ..Reading::decoded()
    };
    let wire = [frames::ID, frames::RATIO].concat();
    assert_eq!(unnamed.encoded(), wire);
    unnamed.assert_decodes_from(&wire);
}

/// A field's own `skip_if` replaces its default's `skip_if_default`: a setpoint of 0 is written,
/// one at room temperature is not, and decodes back through the field's own `default`
#[test]
fn a_fields_own_skip_replaces_its_defaults() {
    let cold = Thermostat {
        offset: 0,
        setpoint: 0,
    };
    // setpoint: tag, varint 0, no length
    let wire = [0x10, 0x00];
    assert_eq!(cold.encoded(), wire);
    cold.assert_decodes_from(&wire);
    let room = Thermostat {
        offset: 0,
        setpoint: Thermostat::ROOM,
    };
    assert_eq!(room.encoded(), b"");
    room.assert_decodes_from(&[]);
}

/// A bare `default` on an `Option` field is the field's own default: an unset schedule writes no
/// frame, and its missing frame decodes back to `None`
#[test]
fn a_bare_default_on_an_option_field_is_none() {
    let unscheduled = Heater { schedule: None };
    assert_eq!(unscheduled.encoded(), b"");
    unscheduled.assert_decodes_from(&[]);
}
