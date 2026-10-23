//! The examples the BER standards give, each pinned both ways: ITU-T X.690 (02/2021) §8.1.3.4,
//! §8.1.3.5 and §8.19.5, SMPTE ST 336:2017 Annex J, and MISB ST 0601.8 §6.5.2.3
//!
//! Each wire is written as the standard prints it, in binary where it prints binary
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::helpers::DecoderExt;
use tinyklv::dec::ber as decber;
use tinyklv::enc::ber as encber;

// --------------------------------------------------
// constants
// --------------------------------------------------
/// X.690's length examples: §8.1.3.4's L = 38 in the short form, and §8.1.3.5's L = 201 in the
/// long form
const X690_LENGTHS: [(usize, &[u8]); 2] =
    [(38, &[0b0010_0110]), (201, &[0b1000_0001, 0b1100_1001])];

/// The values X.690 §8.19.4 packs the first two arcs of an OID into: the first subidentifier is
/// `(X * 40) + Y`
const SECOND_ARCS: u64 = 40;

/// Object identifier examples, each its subidentifiers and its contents: X.690 §8.19.5's
/// {2 999 3}, and SMPTE ST 336:2017 Annex J's {2 100 3}
const OID_CONTENTS: [([u64; 2], &[u8]); 2] = [
    ([2 * SECOND_ARCS + 999, 3], &[0x88, 0x37, 0x03]),
    ([2 * SECOND_ARCS + 100, 3], &[0x81, 0x34, 0x03]),
];

/// MISB ST 0601.8's BER-OID tag examples (Figures 6-4 to 6-6), in one, two and three bytes
const MISB_TAGS: [(u64, &[u8]); 3] = [
    (98, &[0b0110_0010]),
    (144, &[0b1000_0001, 0b0001_0000]),
    (23298, &[0b1000_0001, 0b1011_0110, 0b0000_0010]),
];

#[test]
/// Tests [`X690_LENGTHS`] both ways
fn x690_length_examples() {
    for (length, wire) in X690_LENGTHS {
        let mut encoded = Vec::new();
        encber::ber_length(length, &mut encoded);
        assert_eq!(encoded, wire, "encoding {length}");
        decber::ber_length.assert_reads(wire, length);
    }
}

#[test]
/// Tests [`OID_CONTENTS`] both ways, each read as one subidentifier after another
fn oid_examples() {
    for (subidentifiers, wire) in OID_CONTENTS {
        let mut encoded = Vec::new();
        for subidentifier in subidentifiers {
            encber::ber_oid(subidentifier, &mut encoded);
        }
        assert_eq!(encoded, wire, "encoding {subidentifiers:?}");
        let mut input = wire;
        for subidentifier in subidentifiers {
            assert_eq!(decber::ber_oid::<u64>(&mut input), Ok(subidentifier));
        }
        assert!(
            input.is_empty(),
            "the cursor after {wire:02X?}: {input:02X?}"
        );
    }
}

#[test]
/// Tests [`MISB_TAGS`] both ways
fn misb_tag_examples() {
    for (tag, wire) in MISB_TAGS {
        let mut encoded = Vec::new();
        encber::ber_oid(tag, &mut encoded);
        assert_eq!(encoded, wire, "encoding {tag}");
        decber::ber_oid::<u64>.assert_reads(wire, tag);
    }
}
