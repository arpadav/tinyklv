//! Shared framing primitives.
//!
//! These operate on raw `&[u8]` and belong to no single approach: serde_klv, tlv_parser,
//! and manual all reach the value body through [`seek`], and the framed-decode bench input
//! is built by [`make_noisy`] over [`frame`]. tinyklv frames natively and overrides those
//! defaults, but the sentinel + 1-byte-length envelope produced here is identical to its,
//! so the framed groups compare like for like.
//!
//! Author: aav
// --------------------------------------------------
// external
// --------------------------------------------------
use rand::{RngExt, SeedableRng, rngs::SmallRng};

/// The 2-byte frame sentinel shared by every approach.
pub(crate) const SENTINEL: [u8; 2] = [0x47, 0x48];

/// Fixed RNG seed for the stream noise. Seeding from a constant makes the junk
/// reproducible run-to-run and byte-identical across every approach, so the framed groups
/// compare like for like.
const NOISE_SEED: [u8; 32] = [0x5A; 32];

/// Junk bytes before the zero padding.
const PREFIX_JUNK: usize = 6;

/// Zero padding immediately before the frame; `seek` must skip it.
const ZERO_PAD: usize = 4;

/// Junk bytes after the frame.
const TRAILING_JUNK: usize = 4;

/// Number of framed packets concatenated into one streamed multi-packet buffer.
///
/// Sized so the streaming decoder performs many feed/seek/decode cycles in a row, making the
/// per-packet buffer-reclamation cost (the drain-vs-cursor difference) measurable rather than
/// lost in noise.
pub(crate) const STREAM_PACKETS: usize = 64;

/// Scans past stream noise to the [`SENTINEL`], reads the 1-byte length, and returns the
/// value body. This is the framing work serde_klv / tlv_parser / manual need but cannot do
/// natively (tinyklv does it internally via `decode_frame`).
///
/// # Arguments
///
/// * `buf` - a noisy buffer that contains one framed record.
///
/// # Returns
///
/// The value body slice, or `None` if the sentinel/length/body are not all present.
pub(crate) fn seek(buf: &[u8]) -> Option<&[u8]> {
    seek_next(buf).map(|(body, _)| body)
}

/// Wraps a value body in a frame: [`SENTINEL`] + 1-byte length + body. No stream noise.
///
/// # Arguments
///
/// * `body` - the value body to frame (must be at most 255 bytes).
///
/// # Returns
///
/// The framed bytes.
pub(crate) fn frame(body: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(body.len() + 3);
    out.extend_from_slice(&SENTINEL);
    out.push(u8::try_from(body.len()).expect("frame body exceeds 255 bytes"));
    out.extend_from_slice(body);
    out
}

/// Embeds a frame in realistic stream noise: seeded-random junk + zero padding before it,
/// and trailing junk after. The seed is fixed ([`NOISE_SEED`]), so the noise is identical
/// for every approach and reproducible across runs.
///
/// # Arguments
///
/// * `frame` - a framed record (typically from [`frame`] or a native `encode_frame`).
///
/// # Returns
///
/// The frame surrounded by noise, ready for a framed decode.
pub(crate) fn make_noisy(frame: &[u8]) -> Vec<u8> {
    // --------------------------------------------------
    // seed rng and allocate buffer
    // --------------------------------------------------
    let mut rng = SmallRng::from_seed(NOISE_SEED);
    let mut buf = Vec::with_capacity(frame.len() + PREFIX_JUNK + ZERO_PAD + TRAILING_JUNK);
    // --------------------------------------------------
    // prepend junk, zero padding, then the frame
    // --------------------------------------------------
    push_junk(&mut buf, &mut rng, PREFIX_JUNK);
    buf.extend_from_slice(&[0u8; ZERO_PAD]);
    buf.extend_from_slice(frame);
    // --------------------------------------------------
    // append trailing junk
    // --------------------------------------------------
    push_junk(&mut buf, &mut rng, TRAILING_JUNK);
    buf
}

/// Appends `n` seeded-random junk bytes, excluding the sentinel's lead byte so the junk
/// can never forge a false sentinel ahead of the real frame
///
/// Any byte that equals `SENTINEL[0]` is flipped by XOR with `0x01` before
/// being pushed, ensuring no false sentinel appears in the noise
///
/// # Arguments
///
/// * `buf` - the output buffer to append junk bytes to
/// * `rng` - a seeded RNG used to generate the random bytes
/// * `n` - the number of junk bytes to append
fn push_junk(buf: &mut Vec<u8>, rng: &mut SmallRng, n: usize) {
    for _ in 0..n {
        let mut byte: u8 = rng.random();
        if byte == SENTINEL[0] {
            byte ^= 0x01;
        }
        buf.push(byte);
    }
}

/// Builds a multi-packet noisy stream: [`STREAM_PACKETS`] copies of `frame`, each wrapped in the
/// same junk + zero padding as [`make_noisy`], concatenated into one buffer.
///
/// Models a channel carrying back-to-back frames with inter-frame noise: a streaming decoder must
/// seek past junk to each successive sentinel, decode one packet, then reclaim the consumed bytes
/// before the next. This is the input the `streamed` decode tier times.
///
/// # Arguments
///
/// * `frame` - one full frame (sentinel + length + value body) to repeat.
///
/// # Returns
///
/// A [`Vec<u8>`] holding [`STREAM_PACKETS`] noisy framed packets back to back.
pub(crate) fn make_stream(frame: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    for _ in 0..STREAM_PACKETS {
        out.extend_from_slice(&make_noisy(frame));
    }
    out
}

/// Seeks to the next [`SENTINEL`] in `buf`, returning the value body slice and the remaining
/// buffer immediately after that packet's value.
///
/// The shared (non-tinyklv) `decode_streamed` loops on this to pull successive packets out of a
/// [`make_stream`] buffer: each call skips inter-frame junk to the sentinel, reads the 1-byte
/// length, and hands back the body plus the tail to continue from. Returns `None` once no further
/// complete packet remains.
///
/// # Arguments
///
/// * `buf` - the remaining stream buffer to scan from.
///
/// # Returns
///
/// `Some((body, rest))` for the next packet, or `None` if no further complete packet exists.
pub(crate) fn seek_next(buf: &[u8]) -> Option<(&[u8], &[u8])> {
    // --------------------------------------------------
    // locate sentinel, then read its 1-byte length
    // --------------------------------------------------
    let pos = buf.windows(SENTINEL.len()).position(|w| w == SENTINEL)?;
    let len_idx = pos + SENTINEL.len();
    let len = usize::from(*buf.get(len_idx)?);
    // --------------------------------------------------
    // slice exactly `len` value bytes; tail continues after
    // --------------------------------------------------
    let start = len_idx + 1;
    let body = buf.get(start..start + len)?;
    let rest = &buf[start + len..];
    Some((body, rest))
}
