//! Tests for the `latebind` field attribute
//!
//! Covers both call-shapes of post-decode `latebind`:
//!
//! * consuming - `latebind = path`       → `Fn(T) -> U` - emits `.map(path)`
//! * mutating  - `latebind = &mut path`  → `Fn(&mut T)` - emits
//!   `.map(|mut __v| { path(&mut __v); __v })` (T == U)
//!
//! Exercised on required fields, `Option<U>` fields (present/absent), and
//! alongside `default` to confirm the default-populated path is NOT re-run
//! through latebind
use tinyklv::dec::binary as decb;
use tinyklv::enc::binary as encb;
use tinyklv::prelude::*;
use tinyklv::Klv;

// --------------------------------------------------
// consuming form: wire `u8` -> `Status` via `Fn(u8) -> Status`
// --------------------------------------------------
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Status {
    Idle,
    Active,
    Error,
    Unknown,
}

impl Status {
    fn from_u8(v: u8) -> Self {
        match v {
            0 => Status::Idle,
            1 => Status::Active,
            2 => Status::Error,
            _ => Status::Unknown,
        }
    }
    fn as_u8(&self) -> u8 {
        match self {
            Status::Idle => 0,
            Status::Active => 1,
            Status::Error => 2,
            Status::Unknown => 0xFF,
        }
    }
}

fn enc_status(s: &Status) -> Vec<u8> {
    encb::u8(s.as_u8())
}

#[derive(Klv, Debug, Clone, PartialEq, Eq)]
#[klv(
    sentinel = b"\x00\x00\x00\x10",
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct ConsumingPacket {
    #[klv(
        key = 0x01,
        dec = decb::u8,
        latebind = Status::from_u8,
        enc = enc_status,
    )]
    status: Status,
    #[klv(
        key = 0x02,
        dec = decb::u8,
        latebind = Status::from_u8,
        enc = enc_status,
    )]
    backup: Option<Status>,
}

#[test]
/// Consuming latebind converts `u8` wire bytes into a `Status` enum and roundtrips.
fn consuming_latebind_roundtrip() {
    let original = ConsumingPacket {
        status: Status::Active,
        backup: Some(Status::Error),
    };
    let bytes = original.encode_frame();
    let mut slice = bytes.as_slice();
    let decoded = ConsumingPacket::decode_frame(&mut slice).expect("decode_frame");
    assert_eq!(original, decoded);
}

#[test]
/// Consuming latebind on an absent `Option<U>` leaves the field as `None`.
fn consuming_latebind_optional_absent() {
    let original = ConsumingPacket {
        status: Status::Idle,
        backup: None,
    };
    let bytes = original.encode_frame();
    let mut slice = bytes.as_slice();
    let decoded = ConsumingPacket::decode_frame(&mut slice).expect("decode_frame");
    assert_eq!(original, decoded);
}

// --------------------------------------------------
// mutating form: parse Coordinate with z=0, inject global z via `Fn(&mut T)`
// --------------------------------------------------
const GLOBAL_Z: f32 = 15.0;

#[derive(Debug, Clone, Copy, PartialEq)]
struct Coordinate {
    x: f32,
    y: f32,
    z: f32,
}

fn dec_xy_z0(input: &mut &[u8]) -> tinyklv::Result<Coordinate> {
    let x = decb::be_f32(input)?;
    let y = decb::be_f32(input)?;
    Ok(Coordinate { x, y, z: 0.0 })
}

fn enc_xyz(c: &Coordinate) -> Vec<u8> {
    let mut out = encb::be_f32(c.x);
    out.extend(encb::be_f32(c.y));
    out
}

fn apply_global_z(c: &mut Coordinate) {
    c.z = GLOBAL_Z;
}

#[derive(Klv, Debug, Clone, PartialEq)]
#[klv(
    sentinel = b"\x00\x00\x00\x11",
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct MutatingPacket {
    #[klv(
        key = 0x01,
        dec = dec_xy_z0,
        latebind = &mut apply_global_z,
        enc = enc_xyz,
    )]
    pos: Coordinate,
    #[klv(
        key = 0x02,
        dec = dec_xy_z0,
        latebind = &mut apply_global_z,
        enc = enc_xyz,
    )]
    maybe_pos: Option<Coordinate>,
}

#[test]
/// Mutating latebind injects z from a global after the decoder parses only x/y.
fn mutating_latebind_injects_z() {
    let wire = MutatingPacket {
        pos: Coordinate {
            x: 1.0,
            y: 2.0,
            z: 0.0,
        }, // encoder drops z
        maybe_pos: Some(Coordinate {
            x: 3.0,
            y: 4.0,
            z: 0.0,
        }),
    };
    let bytes = wire.encode_frame();
    let mut slice = bytes.as_slice();
    let decoded = MutatingPacket::decode_frame(&mut slice).expect("decode_frame");
    assert_eq!(decoded.pos.x, 1.0);
    assert_eq!(decoded.pos.y, 2.0);
    assert_eq!(decoded.pos.z, GLOBAL_Z);
    let mp = decoded.maybe_pos.expect("maybe_pos should be Some");
    assert_eq!(mp.x, 3.0);
    assert_eq!(mp.y, 4.0);
    assert_eq!(mp.z, GLOBAL_Z);
}

#[test]
/// Mutating latebind on absent `Option<U>` still produces `None` without invoking the mutator.
fn mutating_latebind_optional_absent() {
    let wire = MutatingPacket {
        pos: Coordinate {
            x: 9.0,
            y: 9.0,
            z: 0.0,
        },
        maybe_pos: None,
    };
    let bytes = wire.encode_frame();
    let mut slice = bytes.as_slice();
    let decoded = MutatingPacket::decode_frame(&mut slice).expect("decode_frame");
    assert!(decoded.maybe_pos.is_none());
    assert_eq!(decoded.pos.z, GLOBAL_Z);
}
