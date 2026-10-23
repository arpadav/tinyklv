//! A fused `keylen` derive emitted by a `macro_rules!` macro, with its field name passed in as an
//! `$x:ident` fragment, its header and value codecs as `$x:path` fragments, or its field type
//! as a `$t:ty` one
//!
//! The derive's generated locals must resolve under the macro's hygiene, on both the decode
//! and the encode side, and a `$t:ty` field type must match the `default(typ = ..)` that names
//! its type: this file failing to compile is the regression
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::common::drive::Drive;
use crate::common::varint;
use crate::support::header;
use tinyklv::prelude::*;

/// Declares a one-field `keylen` message named `$name`, its field named `$field`
macro_rules! reading {
    ($name:ident, $field:ident, $header:path, $header_enc:path, $value:path, $value_enc:path) => {
        /// One-field message declared through [`reading!`]
        #[derive(Klv, Debug, PartialEq)]
        #[klv(keylen(dec = $header, enc = $header_enc))]
        struct $name {
            #[klv(key = 0x02, dec = $value, enc = *$value_enc)]
            /// key 0x02 - `Inline` varint
            $field: u64,
        }
    };
}
reading!(
    Counter,
    count,
    header::dec_u8,
    header::enc_u8,
    varint::dec_u64,
    varint::enc_u64
);

/// Declares a one-field `keylen` message named `$name`, its field a `$ty` coded by the struct's
/// `u64` default
macro_rules! typed_reading {
    ($name:ident, $ty:ty) => {
        /// One-field message declared through [`typed_reading!`]
        #[derive(Klv, Debug, PartialEq)]
        #[klv(
                            keylen(dec = header::dec_u8, enc = header::enc_u8),
                            default(typ = u64, dec = varint::dec_u64, enc = *varint::enc_u64),
                        )]
        struct $name {
            #[klv(key = 0x02)]
            /// key 0x02 - `Inline` varint
            count: $ty,
        }
    };
}
typed_reading!(TypedCounter, u64);

/// The macro-emitted derive encodes and decodes like a hand-written one
#[test]
fn macro_emitted_keylen_derive_roundtrips() {
    #[rustfmt::skip]
    let wire = vec![
        // count = 128 (key 0x02, a two-byte varint)
        0x02, 0x80, 0x01,
    ];
    let counter = Counter { count: 128 };
    assert_eq!(counter.encoded(), wire);
    assert_eq!(Counter::decode_value(&mut &wire[..]), Ok(counter));
}

/// A `$ty:ty` field type takes the codec of the `default(typ = ..)` written out beside it
#[test]
fn macro_type_fragment_takes_its_default() {
    #[rustfmt::skip]
    let wire = vec![
        // count = 128 (key 0x02, a two-byte varint)
        0x02, 0x80, 0x01,
    ];
    let counter = TypedCounter { count: 128 };
    assert_eq!(counter.encoded(), wire);
    assert_eq!(TypedCounter::decode_value(&mut &wire[..]), Ok(counter));
}
