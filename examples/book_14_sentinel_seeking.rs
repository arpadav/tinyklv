#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(clippy::unwrap_used)]
//! See: `book/tutorial/14-sentinel-seeking.md` for full example
use tinyklv::prelude::*;            // Klv proc-macro + traits
use tinyklv::dec::binary as decb;   // binary decoders
use tinyklv::enc::binary as encb;   // binary encoders

#[derive(Klv, Debug, PartialEq, Clone)]
#[klv(
    stream = &[u8],
    sentinel = b"HEARTBEAT",
    key(dec = decb::u8,          enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct HeartbeatPacket {
    #[klv(key = 0x01, dec = decb::u8,     enc = *encb::u8)]     sequence:             u8,
    #[klv(key = 0x02, dec = decb::be_u16, enc = *encb::be_u16)] temperature_centideg: u16,
    #[klv(key = 0x03, dec = decb::be_u32, enc = *encb::be_u32)] uptime_s:             u32,
}

fn main() {
    // three heartbeats in one capture session
    let originals = vec![
        HeartbeatPacket { sequence: 1, temperature_centideg: 2300, uptime_s: 10 },
        HeartbeatPacket { sequence: 2, temperature_centideg: 2310, uptime_s: 20 },
        HeartbeatPacket { sequence: 3, temperature_centideg: 2340, uptime_s: 30 },
    ];

    // glue them into a single noisy buffer: junk, frame, junk, frame, junk, frame, junk
    let mut buffer: Vec<u8> = Vec::new();
    let noise = [0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x00];
    for packet in &originals {
        buffer.extend_from_slice(&noise);
        buffer.extend_from_slice(&packet.encode_frame());
    }
    buffer.extend_from_slice(&noise);

    // pipeline: loop seek_sentinel, track the per-frame starting offset,
    // then decode_value on the returned body sub-stream
    let mut cursor = buffer.as_slice();
    let mut offsets: Vec<usize> = Vec::new();
    let mut decoded: Vec<HeartbeatPacket> = Vec::new();
    let buffer_base = buffer.as_ptr() as usize;

    while let Ok(mut body) = HeartbeatPacket::seek_sentinel(&mut cursor) {
        // at this point `cursor` is past sentinel+len+body, and `body` is
        // the isolated body region; record where the frame began in the
        // original buffer for telemetry or logging
        let remaining_start = cursor.as_ptr() as usize;
        let body_start      = body.as_ptr() as usize;
        let frame_offset    = body_start - buffer_base - b"HEARTBEAT".len() - 1;
        offsets.push(frame_offset);

        // run the value decoder on the isolated body - errors do not bleed
        // back into the outer cursor, so we can keep looping
        let packet = HeartbeatPacket::decode_value(&mut body).unwrap();
        decoded.push(packet);

        // sanity: after decode_value consumed the body, the outer cursor
        // still points at the same position (seek_sentinel already advanced it)
        assert_eq!(cursor.as_ptr() as usize, remaining_start);
    }

    // three frames located, three packets decoded, in wire order
    assert_eq!(decoded, originals);
    assert_eq!(offsets.len(), 3);
    // the first frame starts just after the first 6-byte noise prefix
    assert_eq!(offsets[0], noise.len());
}
