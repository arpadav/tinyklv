#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(clippy::unwrap_used)]
//! Example 07 - `SeekSentinel` scanning past leading junk.
//!
//! Real network streams arrive wrapped in UDP/IP headers, partial payloads
//! from earlier frames, and general noise. `decode_frame` handles this by
//! calling `SeekSentinel::seek_sentinel` internally, scanning forward until
//! it finds the magic bytes, then reading the length and value region. This
//! example plants obvious garbage before a valid frame and asserts that the
//! decoder still recovers the struct unchanged.
//!
//! Showcases:
//! * `decode_frame` skipping arbitrary prefix bytes
//! * Constructing a noisy buffer to exercise the seeker
//! * Post-decode slice position (what the stream looks like after a frame)
use tinyklv::prelude::*;            // Klv proc-macro + traits
use tinyklv::dec::binary as decb;   // binary decoders
use tinyklv::enc::binary as encb;   // binary encoders

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"VIDEOMETA",
    key(dec = decb::u8,          enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
/// Video-metadata tag set embedded inside a larger UDP payload
struct VideoMeta {
    #[klv(
        key = 0x01,
        dec = decb::be_u32,
        enc = *encb::be_u32,
    )]
    /// Monotonic frame sequence number
    frame_seq: u32,

    #[klv(
        key = 0x02,
        dec = decb::be_u16,
        enc = *encb::be_u16,
    )]
    /// Codec bitrate in kbit/s
    bitrate_kbps: u16,
}

fn main() {
    // build - a known-good video-metadata record
    let original = VideoMeta {
        frame_seq:    12_345,
        bitrate_kbps:  4_096,
    };

    // encode - the clean KLV frame, sentinel-prefixed
    let clean_frame = original.encode_frame();

    // prepend noise + zeros the seeker must skip
    let mut noisy_buffer: Vec<u8> = Vec::new();

    // UDP-like preamble junk (not the sentinel)
    noisy_buffer.extend_from_slice(&[
        // IP-version-ish junk:
            0x00, 0x45,
        // bogus source port (5000 BE):
            0x13, 0x88,
        // fake partial checksum:
            0xFF, 0xFE,
    ]);

    // zero padding to exercise the "skip leading zeros" path
    noisy_buffer.extend_from_slice(&[
        // padding zeros:
            0x00, 0x00, 0x00, 0x00,
    ]);

    // the real sentinel-prefixed frame follows the junk
    noisy_buffer.extend_from_slice(&clean_frame);

    // decode - seek_sentinel steps past the 10 prefix bytes and lands on VIDEOMETA
    let mut slice = noisy_buffer.as_slice();
    let decoded = VideoMeta::decode_frame(&mut slice).unwrap();

    // assert - the seeker skipped the junk and reconstructed the payload
    assert_eq!(decoded, original);

    // assert - the entire frame was consumed; only unconsumed tail remains
    assert!(
        slice.is_empty(),
        "clean frame bytes should all have been consumed",
    );
}
