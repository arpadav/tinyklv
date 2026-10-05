//! A fused `keylen` derive emitted by a `macro_rules!` macro, with its header and value
//! codecs passed in as `$x:path` fragments
//!
//! The derive's generated locals must resolve under the macro's hygiene, on both the decode
//! and the encode side: this file failing to compile is the regression
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use tinyklv::dec::ber as decber;
use tinyklv::dec::binary as decb;
use tinyklv::enc::ber as encber;
use tinyklv::prelude::*;

/// Decodes a frame header: a `u8` key, every value `Inline`
///
/// # Arguments
///
/// * `input` - the byte stream positioned at a frame header; advanced past the key
///
/// # Returns
///
/// `Ok((key, Framing::Inline))`, or an error on empty input
fn header(input: &mut &[u8]) -> tinyklv::Result<(u8, Framing)> {
    Ok((decb::u8(input)?, Framing::Inline))
}

/// Encodes a frame header, the inverse of [`header`]: the `u8` key alone, since every value
/// is `Inline` and its length never reaches the wire
///
/// # Arguments
///
/// * `(key, _)` - the field key and the (unwritten) byte length of the value that follows
/// * `out` - the buffer the key byte is appended to
fn header_enc((key, _): (u8, usize), out: &mut Vec<u8>) {
    out.push(key);
}

/// Declares a one-field `keylen` message named `$name`
macro_rules! reading {
    ($name:ident, $header:path, $header_enc:path, $value:path, $value_enc:path) => {
        /// One-field message declared through [`reading!`]
        #[derive(Klv, Debug, PartialEq)]
        #[klv(keylen(dec = $header, enc = $header_enc))]
        struct $name {
            #[klv(key = 0x02, dec = $value, enc = *$value_enc)]
            /// key 0x02 - `Inline` BER-OID
            count: u64,
        }
    };
}
reading!(
    Counter,
    header,
    header_enc,
    decber::ber_oid,
    encber::ber_oid
);

/// The macro-emitted derive encodes and decodes like a hand-written one
#[test]
fn macro_emitted_keylen_derive_roundtrips() {
    #[rustfmt::skip]
    let wire = vec![
        // count = 128 (key 0x02, a two-byte BER-OID)
        0x02, 0x81, 0x00,
    ];
    let counter = Counter { count: 128 };
    let mut out = Vec::new();
    counter.encode_value(&mut out);
    assert_eq!(out, wire);
    assert_eq!(Counter::decode_value(&mut &wire[..]), Ok(counter));
}
