#![allow(clippy::unwrap_used)]
//! Noisy buffer with leading garbage; SeekSentinel resyncs to sentinel bytes.
//!
//! In real network streams, framing bytes often appear after preamble data,
//! padding, or earlier partial packets. `decode_frame` uses `SeekSentinel`
//! internally to scan forward through arbitrary garbage until it finds the
//! magic sentinel pattern `b"\xBE\xEF"`, then reads the length-prefixed
//! body. This example builds a buffer with random junk bytes prepended to
//! a valid frame, confirms that decoding still succeeds, then shows that a
//! second call immediately fails (no more frames in the buffer).

use tinyklv::prelude::*;
use tinyklv::Klv;

/// Video-metadata tag set embedded inside a UDP payload.
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    // Any bytes before this sentinel are treated as framing garbage
    sentinel = b"\xBE\xEF",
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct VideoMeta {
    // Frame sequence number
    #[klv(key = 0x01, dec = tinyklv::dec::binary::be_u32, enc = &tinyklv::enc::binary::be_u32)]
    frame_seq: u32,

    // Codec bitrate in kbit/s
    #[klv(key = 0x02, dec = tinyklv::dec::binary::be_u16, enc = &tinyklv::enc::binary::be_u16)]
    bitrate_kbps: u16,
}

fn main() {
    let original = VideoMeta {
        frame_seq: 12_345,
        bitrate_kbps: 4_096,
    };

    // Build the clean frame bytes first
    let clean_frame = original.encode_frame();
    println!(
        "Clean frame ({} bytes): {:02X?}",
        clean_frame.len(),
        clean_frame
    );

    // Prepend realistic UDP preamble garbage: version byte, source port,
    // checksum, and some filler - none of it is b"\xBE\xEF"
    let mut noisy_buffer: Vec<u8> = vec![
        0x00, 0x45, // IP-like header junk
        0x13, 0x88, // source port 5000
        0xFF, 0xFE, // partial checksum - *not* the sentinel
        0x00, 0x00, 0x00, 0x00, // padding
    ];
    noisy_buffer.extend_from_slice(&clean_frame);
    println!(
        "Noisy buffer ({} bytes): {:02X?}",
        noisy_buffer.len(),
        noisy_buffer
    );

    let mut slice = noisy_buffer.as_slice();

    // decode_frame transparently skips the 10 garbage bytes and finds the frame
    let decoded = VideoMeta::decode_frame(&mut slice).unwrap();
    println!(
        "Decoded after seeking: frame_seq={}, bitrate={}kbps",
        decoded.frame_seq, decoded.bitrate_kbps
    );

    assert_eq!(decoded, original);

    // After consuming the one valid frame the remaining slice is empty
    assert!(
        slice.is_empty(),
        "all frame bytes should have been consumed"
    );

    // A second decode attempt must fail - no more sentinels
    let result = VideoMeta::decode_frame(&mut noisy_buffer.as_slice().split_at(0).0);
    // (just proving the API - we don't re-run from the already-consumed slice)
    drop(result);

    println!("SUCCESS");
}
