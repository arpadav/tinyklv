#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(clippy::unwrap_used)]
//! See: `book/tutorial/10-init-fallback.md` for full example
use tinyklv::prelude::*;            // Klv proc-macro + traits
use tinyklv::dec::binary as decb;   // binary decoders
use tinyklv::enc::binary as encb;   // binary encoders

#[derive(Debug, PartialEq, Clone, Copy)]
/// Operating mode, wire-encoded as a single byte
enum Mode {
    Idle,
    Active,
    Error,
}
impl Mode {
    /// Consuming latebind target: u8 wire value to `Mode`
    fn from_u8(v: u8) -> Self {
        match v {
            0 => Mode::Idle,
            1 => Mode::Active,
            _ => Mode::Error,
        }
    }
}
impl EncodeValue<Vec<u8>> for Mode {
    /// Encoder takes `&Mode`; field attribute uses `enc = Mode::encode_value` (no sigil)
    fn encode_value(&self) -> Vec<u8> {
        encb::u8(match self {
            Mode::Idle   => 0,
            Mode::Active => 1,
            Mode::Error  => 2,
        })
    }
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"HEARTBEAT",
    key(dec = decb::u8,          enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
/// Legacy transmitter: earlier firmware, never sends `signal_dbm` or
/// `battery_pct` keys. Same sentinel and wire grammar as `HeartbeatPacket`
struct HeartbeatLegacy {
    #[klv(key = 0x01, dec = decb::u8,     enc = *encb::u8)]     sequence:             u8,
    #[klv(key = 0x02, dec = decb::be_u16, enc = *encb::be_u16)] temperature_centideg: u16,
    #[klv(key = 0x03, dec = decb::be_u32, enc = *encb::be_u32)] uptime_s:             u32,
    #[klv(key = 0x04, dec = decb::u8, enc = Mode::encode_value, latebind = Mode::from_u8)]
    mode: Mode,
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"HEARTBEAT",
    key(dec = decb::u8,          enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
/// New receiver-side struct: same sentinel as `HeartbeatLegacy` plus two
/// new fields. Works on old and new transmitters alike
struct HeartbeatPacket {
    #[klv(key = 0x01, dec = decb::u8,     enc = *encb::u8)]     sequence:             u8,
    #[klv(key = 0x02, dec = decb::be_u16, enc = *encb::be_u16)] temperature_centideg: u16,
    #[klv(key = 0x03, dec = decb::be_u32, enc = *encb::be_u32)] uptime_s:             u32,
    #[klv(key = 0x04, dec = decb::u8, enc = Mode::encode_value, latebind = Mode::from_u8)]
    mode: Mode,

    /// Absent on the wire = `None`. Encoding a `None` emits no KLV triple
    #[klv(
        key = 0x05,
        dec = decb::i8,
        enc = *encb::i8,
    )]
    signal_dbm: Option<i8>,

    /// Absent on the wire = `100` (the init expression). Always emitted on encode
    #[klv(
        key = 0x06,
        dec = decb::u8,
        enc = *encb::u8,
        init = 100_u8,
    )]
    battery_pct: u8,
}

fn main() {
    // a legacy transmitter emits a short frame with no signal/battery keys
    let legacy = HeartbeatLegacy {
        sequence:             42,
        temperature_centideg: 2350,
        uptime_s:             3600,
        mode:                 Mode::Active,
    };
    let legacy_frame = legacy.encode_frame();

    // the new receiver decodes the legacy frame into the new struct shape;
    // the two absent fields fall back as described above
    let decoded = HeartbeatPacket::decode_frame(
        &mut legacy_frame.as_slice(),
    ).unwrap();

    // signal_dbm was never on the wire -> Option stayed None
    assert_eq!(decoded.signal_dbm, None);
    // battery_pct was never on the wire -> init expression 100 survived
    assert_eq!(decoded.battery_pct, 100);
    // the fields that WERE on the wire decoded normally
    assert_eq!(decoded.sequence,             42);
    assert_eq!(decoded.temperature_centideg, 2350);
    assert_eq!(decoded.uptime_s,             3600);
    assert_eq!(decoded.mode,                 Mode::Active);

    // round-trip check: a full packet survives encode+decode intact
    let full = HeartbeatPacket {
        sequence:             42,
        temperature_centideg: 2350,
        uptime_s:             3600,
        mode:                 Mode::Active,
        signal_dbm:           Some(-54),
        battery_pct:          87,
    };
    let full_frame = full.encode_frame();
    let full_decoded = HeartbeatPacket::decode_frame(
        &mut full_frame.as_slice(),
    ).unwrap();
    assert_eq!(full_decoded, full);

    // when signal_dbm is None the encoded frame is strictly shorter -
    // a `None` field emits no KLV triple
    let thin = HeartbeatPacket {
        signal_dbm: None,
        ..full
    };
    let thin_frame = thin.encode_frame();
    assert!(thin_frame.len() < full_frame.len());
}
