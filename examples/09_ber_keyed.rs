#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(clippy::unwrap_used)]
//! Example 09 - BER-OID keys and BER-encoded lengths.
//!
//! Basic Encoding Rules (BER) give a compact, variable-width representation
//! where values below 128 fit in a single byte and larger values grow by
//! one byte at a time. Many industrial and military protocols use BER-OID
//! for tag keys because they need a huge, extensible tag namespace without
//! paying a fixed two- or four-byte cost on every packet.
//!
//! This example builds a process-control packet with three fields whose
//! BER-OID keys span 1, 2, and 3 bytes, plus a BER length codec so
//! small value regions also stay compact.
//!
//! Showcases:
//! * BER-OID key codec from `dec::ber` / `enc::ber`
//! * BER length codec wrapped in small helper fns
//! * `u64` key type carrying values that map to 1/2/3-byte keys
use tinyklv::prelude::*;            // Klv proc-macro + traits
use tinyklv::dec::binary as decb;   // binary decoders
use tinyklv::enc::binary as encb;   // binary encoders
use tinyklv::dec::ber as decber;    // BER decoders
use tinyklv::enc::ber as encber;    // BER encoders

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"PROCESSCTL",
    key(dec = decber::ber_oid::<u64>, enc = encber::ber_oid),        // variable-width BER-OID keys
    len(dec = decber::ber_length,     enc = encber::ber_length),        // variable-width BER lengths
)]
/// Process-control packet whose tag keys deliberately span different BER widths
struct ProcessControl {
    #[klv(
        key = 0x01_u64,
        dec = decb::u8,
        enc = *encb::u8,
    )]
    /// Single-byte BER key (value < 128)
    device_class: u8,

    #[klv(
        key = 0x0100_u64,
        dec = decb::be_u32,
        enc = *encb::be_u32,
    )]
    /// Two-byte BER key: 256 encodes as `[0x82, 0x00]`
    set_point: u32,

    #[klv(
        key = 0x4001_u64,
        dec = decb::be_u16,
        enc = *encb::be_u16,
    )]
    /// Three-byte BER key: 16385 encodes as `[0x81, 0x80, 0x01]`
    alarm_code: u16,
}

fn main() {
    // build - one field per BER width class
    let original = ProcessControl {
        device_class: 0x05,
        set_point:    75_000, // e.g. flow setpoint in ml/min
        alarm_code:   0x00FF, // warning bit pattern
    };

    // encode - bytes include BER keys of 1, 2, and 3 bytes respectively
    let mut frame = Vec::new();
    original.encode_frame(&mut frame);

    // decode - round-trip through the BER codecs
    let decoded = ProcessControl::decode_frame(
        &mut frame.as_slice(),
    ).unwrap();

    // assert - all three BER-keyed fields survive
    assert_eq!(decoded, original);
}
