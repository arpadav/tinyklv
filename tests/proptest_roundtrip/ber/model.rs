//! [`X690`]: the BER length and BER-OID rules of ITU-T X.690, written as a slow reference the
//! shipped codecs are checked against, and [`Outcome`], what a decoder did with one wire
//!
//! The rules are X.690 (02/2021) §8.1.3 (length octets) and §8.19.2 (one subidentifier), as
//! SMPTE ST 336 adopts them for KLV. Two limits are the codecs' own, not the standard's: a value
//! must fit the decoder's target type, and a BER-OID run holds at most `ceil(bits / 7)` octets,
//! leading `0x80` octets included
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use tinyklv::ErrorKind;

// --------------------------------------------------
// external
// --------------------------------------------------
use num_traits::AsPrimitive;
use std::iter;

// --------------------------------------------------
// constants
// --------------------------------------------------
/// Bit 8 of an octet: set on a long-form length's initial octet (X.690 §8.1.3.5 a)), and on a
/// BER-OID octet another follows (§8.19.2)
pub(super) const BIT_8: u8 = 0x80;

/// Bits 7 to 1 of an octet: a short-form length (X.690 §8.1.3.4), a long form's count of
/// subsequent octets (§8.1.3.5 b)), one BER-OID group (§8.19.2)
pub(super) const BITS_7_TO_1: u8 = !BIT_8;

/// The value bits one BER-OID octet carries (X.690 §8.19.2)
pub(super) const GROUP_BITS: u32 = BITS_7_TO_1.count_ones();

/// The long-form initial octet X.690 §8.1.3.5 c) reserves: every bit set
const RESERVED: u8 = BIT_8 | BITS_7_TO_1;

/// The octets a length's initial octet takes (X.690 §8.1.3.4, §8.1.3.5)
const INITIAL_OCTETS: usize = 1;

/// What a decoder did with one wire
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Outcome {
    /// Read `value` from the first `consumed` octets, the cursor after them
    Reads {
        /// The value read
        value: u128,
        /// The octets read
        consumed: usize,
    },
    /// Rejected the wire as a malformed value, the cursor rewound to its start
    Rewinds,
    /// Ran out of octets, the cursor at the end of the wire
    RunsOut,
    /// Failed any other way, which no rule allows
    Breaks {
        /// The error's kind
        kind: Option<ErrorKind>,
        /// The octets the cursor moved past
        consumed: usize,
    },
}
/// [`Outcome`] implementation
impl Outcome {
    /// Runs `decode` over `wire` and records what it did
    ///
    /// # Arguments
    ///
    /// * `wire` - all the decoder sees
    /// * `decode` - the decoder under test
    pub(super) fn of<T: AsPrimitive<u128>>(
        wire: &[u8],
        decode: impl FnOnce(&mut &[u8]) -> tinyklv::Result<T>,
    ) -> Self {
        let mut cursor = wire;
        let result = decode(&mut cursor);
        let consumed = wire.len() - cursor.len();
        match result {
            Ok(value) => Self::Reads {
                value: value.as_(),
                consumed,
            },
            Err(error) => match ErrorKind::of(&error) {
                Some(ErrorKind::MalformedValue) if consumed == 0 => Self::Rewinds,
                Some(ErrorKind::Truncated) if cursor.is_empty() => Self::RunsOut,
                kind => Self::Breaks { kind, consumed },
            },
        }
    }
}

/// The X.690 rules for one BER length and one BER-OID subidentifier
pub(super) struct X690;
/// [`X690`] implementation
impl X690 {
    /// Decodes a length (X.690 §8.1.3) into a `bits`-wide type
    ///
    /// # Arguments
    ///
    /// * `wire` - the octets, the length first
    /// * `bits` - the width of the decoder's target type
    pub(super) fn length(wire: &[u8], bits: u32) -> Outcome {
        let Some(&initial) = wire.first() else {
            return Outcome::RunsOut;
        };
        // --------------------------------------------------
        // §8.1.3.4 short form: bit 8 zero, bits 7 to 1 the length
        // --------------------------------------------------
        if initial & BIT_8 == 0 {
            return Outcome::Reads {
                value: u128::from(initial),
                consumed: INITIAL_OCTETS,
            };
        }
        // --------------------------------------------------
        // §8.1.3.6.1: `0x80` is the indefinite form, no length; §8.1.3.5 c):
        // `0xFF` shall not be used
        // --------------------------------------------------
        if initial == BIT_8 || initial == RESERVED {
            return Outcome::Rewinds;
        }
        // --------------------------------------------------
        // §8.1.3.5 b): bits 7 to 1 count the subsequent octets, which
        // encode the length, bit 8 of the first most significant. NOTE 2:
        // a sender may use more of them than the minimum
        // --------------------------------------------------
        let count = usize::from(initial & BITS_7_TO_1);
        let Some(subsequent) = wire.get(INITIAL_OCTETS..INITIAL_OCTETS + count) else {
            return Outcome::RunsOut;
        };
        let value = subsequent.iter().try_fold(0_u128, |value, &octet| {
            value
                .checked_mul(1 << u8::BITS)
                .map(|value| value | u128::from(octet))
        });
        match value.filter(|&value| Self::fits(value, bits)) {
            Some(value) => Outcome::Reads {
                value,
                consumed: INITIAL_OCTETS + count,
            },
            None => Outcome::Rewinds,
        }
    }

    /// Decodes one subidentifier (X.690 §8.19.2) into a `bits`-wide type
    ///
    /// # Arguments
    ///
    /// * `wire` - the octets, the subidentifier first
    /// * `bits` - the width of the decoder's target type
    pub(super) fn oid(wire: &[u8], bits: u32) -> Outcome {
        let cap = bits.div_ceil(GROUP_BITS) as usize;
        let mut value = Some(0_u128);
        for (index, &octet) in wire.iter().take(cap).enumerate() {
            // --------------------------------------------------
            // bits 7 to 1 of each octet, the first most significant
            // --------------------------------------------------
            value = value
                .and_then(|value| value.checked_mul(1 << GROUP_BITS))
                .map(|value| value | u128::from(octet & BITS_7_TO_1));
            // --------------------------------------------------
            // bit 8 is zero on the last octet of the series
            // --------------------------------------------------
            if octet & BIT_8 == 0 {
                return match value.filter(|&value| Self::fits(value, bits)) {
                    Some(value) => Outcome::Reads {
                        value,
                        consumed: index + 1,
                    },
                    None => Outcome::Rewinds,
                };
            }
        }
        if wire.len() < cap {
            Outcome::RunsOut
        } else {
            Outcome::Rewinds
        }
    }

    /// Encodes a length in the fewest octets (X.690 §8.1.3.4, §8.1.3.5; §10.1 for the fewest)
    ///
    /// # Arguments
    ///
    /// * `value` - the length
    pub(super) fn encode_length(value: u128) -> Vec<u8> {
        if let Ok(short @ 0..BIT_8) = u8::try_from(value) {
            return vec![short];
        }
        let octets: Vec<u8> = value
            .to_be_bytes()
            .into_iter()
            .skip_while(|&octet| octet == 0)
            .collect();
        iter::once(BIT_8 | octets.len() as u8)
            .chain(octets)
            .collect()
    }

    /// Encodes a subidentifier in the fewest octets (X.690 §8.19.2)
    ///
    /// # Arguments
    ///
    /// * `value` - the subidentifier
    pub(super) fn encode_oid(value: u128) -> Vec<u8> {
        let mut octets = vec![(value as u8) & BITS_7_TO_1];
        let mut rest = value >> GROUP_BITS;
        while rest != 0 {
            octets.push(BIT_8 | ((rest as u8) & BITS_7_TO_1));
            rest >>= GROUP_BITS;
        }
        octets.reverse();
        octets
    }

    /// Whether `value` fits a `bits`-wide unsigned type
    ///
    /// # Arguments
    ///
    /// * `value` - the decoded value
    /// * `bits` - the width of the type
    fn fits(value: u128, bits: u32) -> bool {
        value.checked_shr(bits).is_none_or(|high| high == 0)
    }
}
