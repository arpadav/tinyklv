//! The manual approach: "the right way to parse KLV by hand". Each record gets a bespoke
//! key/length/value state machine to decode (see [`flat`] / [`nested`]) and a matching
//! hand-pushed byte sequence to encode. It has no native framing, so it inherits the
//! shared seek-based `decode_framed` - exactly the hand-written seek glue the framed
//! groups measure.
//!
//! Author: aav
// --------------------------------------------------
// mods
// --------------------------------------------------
mod flat;
mod nested;

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::suite::Approach;

/// The hand-written manual approach
///
/// All encoding is performed by pushing key/length/value triples with [`put`]
/// and all decoding is performed by a bespoke tag-dispatch loop that reads
/// the same triples back. No library derive or generic tree is involved
pub(crate) struct Manual;

/// [`Manual`] implementation of [`Approach`]
impl Approach for Manual {
    const NAME: &'static str = "manual";
}

/// Appends one key/length/value triple to the output buffer
///
/// Pushes `key`, then the length of `val` as a single byte, then the value
/// bytes themselves. The length is converted via [`u8::try_from`] rather than
/// `as u8`, so a field body larger than 255 bytes panics loudly instead of
/// silently truncating - impossible for the bench fixtures, but honest about
/// the 1-byte-length wire format's hard limit
///
/// # Arguments
///
/// * `out` - the output buffer to append the triple to
/// * `key` - the 1-byte field key
/// * `val` - the field value body; must be at most 255 bytes
///
/// # Safety
///
/// Panics via `expect` if `val.len()` exceeds 255 - this reflects a genuine
/// wire-format constraint and is intentional rather than defensive
fn put(out: &mut Vec<u8>, key: u8, val: &[u8]) {
    out.push(key);
    out.push(u8::try_from(val.len()).expect("manual field exceeds 255 bytes"));
    out.extend_from_slice(val);
}
