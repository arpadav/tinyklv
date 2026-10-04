//! A fused `keylen` derive emitted by a `macro_rules!` macro, with its header and value
//! decoders passed in as `$x:path` fragments
//!
//! The derive's generated locals must resolve under the macro's hygiene: this file failing to
//! compile is the regression
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use tinyklv::dec::ber as decber;
use tinyklv::dec::binary as decb;
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

/// Declares a one-field `keylen` message named `$name`
macro_rules! reading {
    ($name:ident, $header:path, $value:path) => {
        /// One-field message declared through [`reading!`]
        #[derive(Klv, Debug, PartialEq)]
        #[klv(keylen(dec = $header), allow_unimplemented_encode)]
        struct $name {
            #[klv(key = 0x02, dec = $value)]
            /// key 0x02 - `Inline` BER-OID
            count: u64,
        }
    };
}
reading!(Counter, header, decber::ber_oid);

/// The macro-emitted derive decodes like a hand-written one
#[test]
fn macro_emitted_keylen_derive_decodes() {
    #[rustfmt::skip]
    let wire = [
        // count = 128 (key 0x02, a two-byte BER-OID)
        0x02, 0x81, 0x00,
    ];
    assert_eq!(
        Counter::decode_value(&mut &wire[..]),
        Ok(Counter { count: 128 })
    );
}
