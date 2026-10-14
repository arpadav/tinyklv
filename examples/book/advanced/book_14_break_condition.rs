//! Book tutorial 14 - container `break_on` for loop control
//! See `book/src/tutorial/advanced/14-break-condition.md` for the full narrative
//!
//! Demonstrates the `#[klv(break_on = ..)]` container attribute, which lets a
//! derived decoder control its key-dispatch loop. The break expression is a
//! function `fn(&key, &mut input) -> BreakType` (a key literal is also accepted
//! as a shorthand for "stop on this key"). Here a reserved key (`0xFE`) is
//! silently skipped, and a terminator key (`0xFF`) stops parsing before the junk
//! bytes that follow. Both encode and decode come from the single derive
use tinyklv::dec::binary as decb; // binary decoders
use tinyklv::enc::binary as encb; // binary encoders
use tinyklv::prelude::*; // Klv proc-macro + traits + BreakType
use winnow::binary::length_take; // a length, then that many bytes

/// Wire keys in use
const KEY_RESERVED: u8 = 0xFE; // consume-and-skip
const KEY_TERMINATOR: u8 = 0xFF; // stop looping

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8,          enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
    break_on = classify,
)]
/// Heartbeat body. The same derive produces both the encoder and a decoder
/// whose loop honours the reserved / terminator keys via `break_on`
struct Heartbeat {
    #[klv(key = 0x01, dec = decb::u8,     enc = *encb::u8)]
    sequence: u8,

    #[klv(key = 0x02, dec = decb::be_u16, enc = *encb::be_u16)]
    temperature_centideg: u16,

    #[klv(key = 0x03, dec = decb::be_u32, enc = *encb::be_u32)]
    uptime_s: u32,
}

/// Classify each decoded key into a loop-control outcome. `break_on = classify`
/// wires this into the generated decode loop: it runs right after every key,
/// with the input after it. A skipped or terminating item's length and value
/// are read here, so the input moves past them
fn classify(key: &u8, input: &mut &[u8]) -> BreakType {
    let outcome = match *key {
        KEY_RESERVED => BreakType::Skip,
        KEY_TERMINATOR => BreakType::Done,
        _ => return BreakType::Proceed,
    };
    match length_take(decb::u8_as_usize).parse_next(input) {
        Ok(_) => outcome,
        Err(_) => BreakType::Proceed,
    }
}

fn main() {
    // produce the canonical three-field body with the derive
    let mut body = Vec::new();
    Heartbeat {
        sequence: 42,
        temperature_centideg: 2350,
        uptime_s: 3600,
    }
    .encode_value(&mut body);

    // reserved triple: key, len, garbage value - the loop must skip it
    #[rustfmt::skip]
    body.extend_from_slice(&[
        KEY_RESERVED, 0x02,
        0xDE, 0xAD,
    ]);
    // terminator triple: key, zero len - the loop must stop here
    #[rustfmt::skip]
    body.extend_from_slice(&[
        KEY_TERMINATOR, 0x00,
    ]);
    // junk after the terminator that must NOT be read
    #[rustfmt::skip]
    body.extend_from_slice(&[
        0xFF, 0xFF, 0xFF, 0xFF,
    ]);

    // decode: Skip drops the reserved bytes, Done stops before the junk
    let mut input = body.as_slice();
    let decoded = Heartbeat::decode_value(&mut input).unwrap();

    assert_eq!(
        decoded,
        Heartbeat {
            sequence: 42,
            temperature_centideg: 2350,
            uptime_s: 3600,
        }
    );
    // the terminator's length was read before `Done`: only the junk is left
    assert_eq!(input, [0xFF, 0xFF, 0xFF, 0xFF]);
}
