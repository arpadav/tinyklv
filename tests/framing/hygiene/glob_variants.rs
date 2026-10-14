//! A key + len derive with a user enum's `None` variant glob-imported into its scope
//!
//! The generated code matches each required field's accumulated `Option` against `Some` and
//! `None`, so a bare `None` there would name [`Compression::None`], which a glob import puts in
//! scope over the prelude's: [`Archive`] must decode and encode as if the import were not there
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::common::drive::Drive;
use Compression::*;
use tinyklv::dec::binary as decb;
use tinyklv::enc::binary as encb;
use tinyklv::prelude::*;

/// How an archive's payload is compressed, its first variant named like `Option::None`
#[derive(Debug, Clone, Copy, PartialEq)]
enum Compression {
    /// stored as is
    None,
    /// zlib-compressed
    Zlib,
}

/// [`Compression`] implementation
impl Compression {
    /// Decodes the codec byte: `0` is [`Compression::None`], any other [`Compression::Zlib`]
    fn decode(input: &mut &[u8]) -> tinyklv::Result<Self> {
        decb::u8(input).map(|byte| if byte == 0 { None } else { Zlib })
    }

    /// Encodes the codec byte
    fn encode(&self, out: &mut Vec<u8>) {
        out.push(u8::from(*self == Zlib));
    }
}

/// An archive header, both fields required
#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct Archive {
    /// key `0x01`, one byte
    #[klv(key = 0x01, dec = Compression::decode, enc = Compression::encode)]
    compression: Compression,

    /// key `0x02`, two bytes
    #[klv(key = 0x02, dec = decb::be_u16, enc = *encb::be_u16)]
    size: u16,
}

/// Both fields decode in both drivers, and the value encodes back
#[test]
fn a_glob_imported_none_variant_leaves_the_derive_alone() {
    let archive = Archive {
        compression: Compression::None,
        size: 300,
    };
    #[rustfmt::skip]
    let wire = [
        // compression = none
        0x01, 0x01, 0x00,
        // size = 300
        0x02, 0x02, 0x01, 0x2C,
    ];
    archive.assert_decodes_from(&wire);
    assert_eq!(archive.encoded(), wire);
}
