#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(clippy::unwrap_used)]
//! See: `book/tutorial/13-break-condition.md` for full example
use tinyklv::prelude::*;            // Klv proc-macro + traits
use tinyklv::dec::binary as decb;   // binary decoders
use tinyklv::enc::binary as encb;   // binary encoders

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8,          enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
/// Encoder-side mirror of the heartbeat body. Used only to produce bytes
/// that the hand-written decoder below will consume
struct HeartbeatEncoder {
    #[klv(key = 0x01, dec = decb::u8,     enc = *encb::u8)]     sequence:             u8,
    #[klv(key = 0x02, dec = decb::be_u16, enc = *encb::be_u16)] temperature_centideg: u16,
    #[klv(key = 0x03, dec = decb::be_u32, enc = *encb::be_u32)] uptime_s:             u32,
}

#[derive(Debug, PartialEq)]
/// Hand-decoded form that honours the terminator / reserved keys
struct HeartbeatDecoded {
    sequence:             u8,
    temperature_centideg: u16,
    uptime_s:             u32,
}

/// Wire keys in use
const KEY_SEQUENCE:    u8 = 0x01;
const KEY_TEMPERATURE: u8 = 0x02;
const KEY_UPTIME:      u8 = 0x03;
const KEY_RESERVED:    u8 = 0xFE; // consume-and-skip
const KEY_TERMINATOR:  u8 = 0xFF; // stop looping

/// Classify a key into one of the four break-condition outcomes. Keeping
/// this function separate makes the loop body below read top-to-bottom
fn classify(key: u8) -> BreakConditionType {
    match key {
        KEY_RESERVED   => BreakConditionType::Skip,
        KEY_TERMINATOR => BreakConditionType::Done,
        _              => BreakConditionType::Proceed,
    }
}

impl DecodeValue<&[u8]> for HeartbeatDecoded {
    fn decode_value(input: &mut &[u8]) -> tinyklv::Result<Self> {
        let mut sequence:    Option<u8>  = None;
        let mut temperature: Option<u16> = None;
        let mut uptime:      Option<u32> = None;

        loop {
            // clean EOF while trying to read the next key ends the loop
            let key = match decb::u8(input) {
                Ok(k) => k,
                Err(_) => break,
            };
            let len = decb::u8_as_usize(input)?;

            // consult the break-condition classifier BEFORE dispatching
            match classify(key) {
                BreakConditionType::Proceed => {}
                BreakConditionType::Skip => {
                    // consume exactly `len` bytes and continue
                    let _ = winnow::token::take::<_, _, winnow::error::ContextError>(len)
                        .parse_next(input)?;
                    continue;
                }
                BreakConditionType::Done     => break,
                BreakConditionType::Abort(e) => return Err(e),
            }

            // proceed: dispatch the value decoder based on key
            match key {
                KEY_SEQUENCE    => sequence    = Some(decb::u8(input)?),
                KEY_TEMPERATURE => temperature = Some(decb::be_u16(input)?),
                KEY_UPTIME      => uptime      = Some(decb::be_u32(input)?),
                _ => {
                    // unknown key - consume and drop
                    let _ = winnow::token::take::<_, _, winnow::error::ContextError>(len)
                        .parse_next(input)?;
                }
            }
        }

        Ok(HeartbeatDecoded {
            sequence:             sequence.unwrap_or_default(),
            temperature_centideg: temperature.unwrap_or_default(),
            uptime_s:             uptime.unwrap_or_default(),
        })
    }
}

fn main() {
    // produce the canonical three-field body with the derive
    let mut body = HeartbeatEncoder {
        sequence:             42,
        temperature_centideg: 2350,
        uptime_s:             3600,
    }.encode_value();

    // splice in a reserved triple (key=0xFE, len=2, garbage) before the
    // terminator - the decoder must silently discard it
    body.extend_from_slice(&[KEY_RESERVED, 0x02, 0xDE, 0xAD]);
    // append a terminator key (len=0) - the decoder must stop here
    body.extend_from_slice(&[KEY_TERMINATOR, 0x00]);
    // anything after the terminator must NOT be read - plant obvious junk
    body.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]);

    // decode: skip drops the reserved bytes, done stops before the junk
    let decoded = HeartbeatDecoded::decode_value(
        &mut body.as_slice(),
    ).unwrap();

    assert_eq!(decoded, HeartbeatDecoded {
        sequence:             42,
        temperature_centideg: 2350,
        uptime_s:             3600,
    });
}
