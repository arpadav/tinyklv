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
use crate::common::drive::Drive;
use crate::support::header;
use tinyklv::dec::ber as decber;
use tinyklv::enc::ber as encber;
use tinyklv::prelude::*;

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
    header::dec_u8,
    header::enc_u8,
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
    assert_eq!(counter.encoded(), wire);
    assert_eq!(Counter::decode_value(&mut &wire[..]), Ok(counter));
}
