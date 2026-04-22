#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(clippy::unwrap_used)]
//! Example 16 - `latebind` post-decode hook, consuming and mutating forms.
//!
//! The `latebind` field attribute runs a second pass on the decoded value
//! immediately after the field's decoder returns. It has two spellings:
//!
//! * **Consuming**: `latebind = path` with signature `Fn(T) -> U`. The
//!   decoder returns `T`, the latebind fn converts it to `U`, and the
//!   struct field is declared as `U`. Useful for turning wire-representation
//!   integers into domain enums.
//!
//! * **Mutating**: `latebind = &mut path` with signature `Fn(&mut T)`. The
//!   decoder returns `T` and the latebind fn mutates it in place. Useful
//!   for injecting data from external context into a partial decode.
//!
//! This example uses the consuming form to build a `Status` enum from a
//! wire `u8`, and the mutating form to inject a Z coordinate that is not
//! carried on the wire at all.
//!
//! Showcases:
//! * `latebind = path` (consuming) `Fn(u8) -> Status`
//! * `latebind = &mut path` (mutating) `Fn(&mut Coordinate)`
//! * Encoding a field whose in-memory type differs from the wire type
//!
//! See also: book Tutorial 16.
use tinyklv::prelude::*;            // Klv proc-macro + traits
use tinyklv::dec::binary as decb;   // binary decoders
use tinyklv::enc::binary as encb;   // binary encoders

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Device status variants carried as a one-byte discriminant on the wire
enum Status {
    Idle,
    Active,
    Error,
    Unknown,
}

/// Consuming-form latebind: convert a wire `u8` into the in-memory `Status`
fn status_from_u8(v: u8) -> Status {
    match v {
        0 => Status::Idle,
        1 => Status::Active,
        2 => Status::Error,
        _ => Status::Unknown,
    }
}

/// Encode a `Status` back to its wire `u8` discriminant
fn enc_status(s: &Status) -> Vec<u8> {
    let byte = match s {
        Status::Idle    => 0,
        Status::Active  => 1,
        Status::Error   => 2,
        Status::Unknown => 0xFF,
    };
    encb::u8(byte)
}

#[derive(Klv, Debug, PartialEq, Eq)]
#[klv(
    stream = &[u8],
    sentinel = b"DEVICE",
    key(dec = decb::u8,          enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
/// Device with a single status field whose wire type differs from the
/// in-memory type; the gap is bridged by a consuming-form latebind
struct Device {
    /// Status discriminant: decoder reads `u8`, latebind promotes to `Status`
    #[klv(
        key = 0x01,
        dec = decb::u8,
        latebind = status_from_u8,
        enc = enc_status,
    )]
    status: Status,
}

/// Z coordinate that is never placed on the wire; injected post-decode
const GLOBAL_Z: f32 = 15.0;

#[derive(Debug, Clone, Copy, PartialEq)]
/// 3D coordinate whose X and Y come from the wire, Z from external context
struct Coordinate {
    x: f32,
    y: f32,
    z: f32,
}

/// Decoder that parses only X and Y, leaving Z as a placeholder zero
fn dec_xy_z0(input: &mut &[u8]) -> tinyklv::Result<Coordinate> {
    let x = decb::be_f32(input)?;
    let y = decb::be_f32(input)?;
    Ok(Coordinate { x, y, z: 0.0 })
}

/// Encoder for X then Y (matches the decoder; Z is never transmitted)
fn enc_xyz(c: &Coordinate) -> Vec<u8> {
    let mut out = encb::be_f32(c.x);
    out.extend(encb::be_f32(c.y));
    out
}

/// Mutating-form latebind: inject the globally-known Z after decoding
fn apply_global_z(c: &mut Coordinate) {
    c.z = GLOBAL_Z;
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"TELEMETRY",
    key(dec = decb::u8,          enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
/// Telemetry frame whose position field is completed by a mutating latebind
struct Telemetry {
    /// Position: decoder reads X and Y, latebind injects Z from global state
    #[klv(
        key = 0x01,
        dec = dec_xy_z0,
        latebind = &mut apply_global_z,
        enc = enc_xyz,
    )]
    position: Coordinate,
}

fn main() {
    // consuming form: round-trip a Status through its u8 wire representation

    // build
    let device = Device { status: Status::Active };
    // encode
    let frame = device.encode_frame();
    // decode
    let decoded = Device::decode_frame(
        &mut frame.as_slice(),
    ).unwrap();
    // assert - the latebind promoted the decoded u8 into the Active variant
    assert_eq!(decoded.status, Status::Active);

    // mutating form: Z is dropped on the wire and re-injected on decode

    // build
    let tele = Telemetry {
        position: Coordinate { x: 1.5, y: 2.5, z: 0.0 },
    };
    // encode - only X and Y are written
    let frame = tele.encode_frame();
    // decode - latebind mutates Z to GLOBAL_Z
    let decoded = Telemetry::decode_frame(
        &mut frame.as_slice(),
    ).unwrap();

    // assert - X and Y round-tripped, Z came from the mutating latebind
    assert_eq!(decoded.position.x, 1.5);
    assert_eq!(decoded.position.y, 2.5);
    assert_eq!(decoded.position.z, GLOBAL_Z);
}
