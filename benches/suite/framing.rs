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
use rand::rngs::SmallRng;
use rand::{RngExt, SeedableRng};

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
    let pos = buf.windows(2).position(|w| w == SENTINEL)?;
    let len_idx = pos + 2;
    let len = usize::from(*buf.get(len_idx)?);
    let start = len_idx + 1;
    buf.get(start..start + len)
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
